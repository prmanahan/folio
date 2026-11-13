//! Spawned-binary harness for `folio` subcommands (plan T3, anchors D8, D10).
//!
//! Black-box: every observation crosses the process boundary (exit code,
//! combined stdout/stderr, the filesystem at DATABASE_URL). Nothing here
//! links the crate under test. No implementation code was read: the env var
//! names (STATIC_DIR, DATABASE_URL, PORT) and the `frontend/build` default
//! come from spec R-18, and the file shapes from the spec's Data Model.
//!
//! Isolation, per R-18: the child runs from an empty temp cwd outside the
//! checkout (no `.env` reachable), with its environment cleared and then
//! given only an absolute STATIC_DIR, a DATABASE_URL naming a file in an
//! empty temp dir, and a PORT this process holds open on 0.0.0.0. The test
//! process mutates no process-global state: env goes to the child only.
#![allow(dead_code)]

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use time::OffsetDateTime;

static COUNTER: AtomicU32 = AtomicU32::new(0);
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

pub struct Sandbox {
    root: PathBuf,
    pub cwd: PathBuf,
    pub static_dir: PathBuf,
    pub db_dir: PathBuf,
    pub db_path: PathBuf,
    held: TcpListener,
}

impl Sandbox {
    pub fn new() -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "folio-spawned-{}-{}-{}",
            std::process::id(),
            n,
            nanos
        ));
        let cwd = root.join("cwd");
        let static_dir = root.join("static");
        let db_dir = root.join("db");
        for d in [&cwd, &static_dir, &db_dir] {
            fs::create_dir_all(d).unwrap();
        }
        let db_path = db_dir.join("site.db");
        let held = TcpListener::bind("0.0.0.0:0").expect("hold a port on 0.0.0.0");
        Sandbox {
            root,
            cwd,
            static_dir,
            db_dir,
            db_path,
            held,
        }
    }

    pub fn port(&self) -> u16 {
        self.held.local_addr().unwrap().port()
    }

    pub fn list_path(&self) -> PathBuf {
        self.static_dir.join("projects.json")
    }

    pub fn write_list(&self, contents: &str) {
        fs::write(self.list_path(), contents).unwrap();
    }

    /// Run `folio <args>` under the R-18 isolation, with STATIC_DIR set.
    pub fn run(&self, args: &[&str]) -> Outcome {
        self.run_with(args, true)
    }

    /// As `run`, but with STATIC_DIR left unset (the default is relative to
    /// the child's cwd).
    pub fn run_without_static_dir(&self, args: &[&str]) -> Outcome {
        self.run_with(args, false)
    }

    fn run_with(&self, args: &[&str], with_static_dir: bool) -> Outcome {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_folio"));
        cmd.args(args)
            .current_dir(&self.cwd)
            .env_clear()
            .env("DATABASE_URL", &self.db_path)
            .env("PORT", self.port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if with_static_dir {
            cmd.env("STATIC_DIR", &self.static_dir);
        }
        let mut child = cmd.spawn().expect("spawn folio");
        let mut out = child.stdout.take().unwrap();
        let mut err = child.stderr.take().unwrap();
        let t_out = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = out.read_to_string(&mut s);
            s
        });
        let t_err = std::thread::spawn(move || {
            let mut s = String::new();
            let _ = err.read_to_string(&mut s);
            s
        });
        let start = Instant::now();
        let status = loop {
            if let Some(st) = child.try_wait().unwrap() {
                break Some(st);
            }
            if start.elapsed() > CHILD_DEADLINE {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        let stdout = t_out.join().unwrap();
        let stderr = t_err.join().unwrap();
        let output = strip_ansi(&format!("{stdout}{stderr}"));
        match status {
            Some(st) => Outcome {
                code: st.code(),
                timed_out: false,
                output,
            },
            None => Outcome {
                code: None,
                timed_out: true,
                output,
            },
        }
    }

    /// No file may exist at DATABASE_URL, and nothing (journal, WAL) may
    /// have appeared beside it.
    pub fn assert_no_database(&self) {
        assert!(
            !self.db_path.exists(),
            "a file exists at DATABASE_URL ({}) after the run",
            self.db_path.display()
        );
        let entries: Vec<_> = fs::read_dir(&self.db_dir)
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            entries.is_empty(),
            "the DATABASE_URL directory is not empty after the run: {entries:?}"
        );
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

pub struct Outcome {
    pub code: Option<i32>,
    pub timed_out: bool,
    /// Combined stdout then stderr, ANSI escapes stripped.
    pub output: String,
}

impl Outcome {
    /// Exit 0, and not a timeout.
    pub fn assert_exit_zero(&self) {
        assert!(!self.timed_out, "child did not exit within the deadline");
        assert_eq!(
            self.code,
            Some(0),
            "expected exit 0, got {:?}; output:\n{}",
            self.code,
            self.output
        );
    }

    /// Non-zero exit that is an error exit rather than a panic, whose
    /// output names `path`. The path is the condition-specific
    /// discriminator that separates "the validator rejected this file" from
    /// "clap does not know the subcommand" (exit 2, never names the file).
    pub fn assert_rejected_naming(&self, path: &Path) {
        assert!(!self.timed_out, "child did not exit within the deadline");
        match self.code {
            Some(0) => panic!("expected non-zero exit, got 0; output:\n{}", self.output),
            Some(_) => {}
            None => panic!("child died on a signal; output:\n{}", self.output),
        }
        assert!(
            !self.output.contains("panicked at"),
            "a panic stands in for an error exit; output:\n{}",
            self.output
        );
        let p = path.to_string_lossy();
        assert!(
            self.output.contains(p.as_ref()),
            "output does not name the path {p}; output:\n{}",
            self.output
        );
    }

    /// As `assert_rejected_naming`, and the output also contains `fetched_at`.
    pub fn assert_rejected_naming_fetched_at(&self, path: &Path) {
        self.assert_rejected_naming(path);
        assert!(
            self.output.contains("fetched_at"),
            "output does not contain `fetched_at`; output:\n{}",
            self.output
        );
    }

    /// R-18 pass line: exit 0, and one line of the combined output that
    /// contains the path, the `fetched_at` literal, the age, the bound and
    /// each owner's count. The spec does not pin how age and bound are
    /// rendered, so each is matched as a digit run on the line once the
    /// path, the literal and the log prefix are removed.
    pub fn assert_pass_line(
        &self,
        path: &Path,
        fetched_at: &str,
        age: AgeRendering,
        bound_minutes: u64,
        counts: &[(&str, usize)],
    ) {
        self.assert_exit_zero();
        let p = path.to_string_lossy().to_string();
        let line = self
            .output
            .lines()
            .find(|l| l.contains(&p) && l.contains(fetched_at))
            .unwrap_or_else(|| {
                panic!(
                    "no single output line contains both the path {p} and the fetched_at \
                     literal {fetched_at}; output:\n{}",
                    self.output
                )
            });
        let rem = remainder(line, &p, fetched_at);
        let runs = digit_runs(&rem);
        assert!(
            runs.iter().any(|r| age.matches(*r)),
            "line does not show the age ({age:?}); digit runs {runs:?}; line: {line}"
        );
        let bound = [bound_minutes, bound_minutes * 60];
        assert!(
            runs.iter().any(|r| bound.contains(r)),
            "line does not show the bound ({bound_minutes} min); digit runs {runs:?}; line: {line}"
        );
        for (owner, count) in counts {
            assert!(
                has_count(&rem, owner, *count),
                "line does not show owner {owner} with count {count}; line: {line}"
            );
        }
    }
}

/// How an age may be rendered: whole minutes, or seconds within a window
/// (the child's start-up delay moves the age a few seconds).
#[derive(Debug, Clone)]
pub struct AgeRendering {
    pub minutes: &'static [u64],
    pub seconds: std::ops::RangeInclusive<u64>,
}

impl AgeRendering {
    fn matches(&self, run: u64) -> bool {
        self.minutes.contains(&run) || self.seconds.contains(&run)
    }
}

/// 29 minutes old at spawn: shown as 29 (minutes) or about 1740 seconds.
pub fn age_29_min_old() -> AgeRendering {
    AgeRendering {
        minutes: &[29],
        seconds: 1735..=1770,
    }
}

/// 4 minutes ahead at spawn: the age is negative; shown as 4, or 3 when
/// floored, or about 240 seconds.
pub fn age_4_min_ahead() -> AgeRendering {
    AgeRendering {
        minutes: &[3, 4],
        seconds: 230..=245,
    }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for n in chars.by_ref() {
                if n.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn remainder(line: &str, path: &str, fetched_at: &str) -> String {
    let rem = line.replace(path, " ").replace(fetched_at, " ");
    // Drop a leading tracing timestamp token (starts with a digit, has a 'T').
    let trimmed = rem.trim_start();
    if let Some(first) = trimmed.split_whitespace().next()
        && first.starts_with(|c: char| c.is_ascii_digit())
        && first.contains('T')
    {
        return trimmed[first.len()..].to_string();
    }
    rem
}

fn digit_runs(s: &str) -> Vec<u64> {
    let mut runs = Vec::new();
    let mut cur = String::new();
    for c in s.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            if let Ok(n) = cur.parse() {
                runs.push(n);
            }
            cur.clear();
        }
    }
    runs
}

/// The owner's name is followed (after separators) or preceded by the count.
fn has_count(rem: &str, owner: &str, count: usize) -> bool {
    let want = count.to_string();
    rem.match_indices(owner).any(|(i, _)| {
        let after: String = rem[i + owner.len()..]
            .chars()
            .skip_while(|c| !c.is_ascii_alphanumeric())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        let mut before_rev: Vec<char> = rem[..i]
            .chars()
            .rev()
            .skip_while(|c| !c.is_ascii_alphanumeric())
            .take_while(|c| c.is_ascii_digit())
            .collect();
        before_rev.reverse();
        let before: String = before_rev.into_iter().collect();
        after == want || before == want
    })
}

/// `fetched_at` in the exact grammar `YYYY-MM-DDTHH:MM:SS.sssZ`, for now
/// plus `offset_secs`. Written with `time`'s `formatting` feature (dev-only).
pub fn fetched_at(offset_secs: i64) -> String {
    let fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z",
    )
    .unwrap();
    (OffsetDateTime::now_utc() + time::Duration::seconds(offset_secs))
        .format(&fmt)
        .unwrap()
}

// ---- list-file builders (JSON text, built in test code only) ----

pub fn repo(name: &str, description_json_body: &str) -> String {
    format!(r#"{{"name":"{name}","description":"{description_json_body}"}}"#)
}

pub fn group(owner: &str, repos: &[String]) -> String {
    format!(r#"{{"owner":"{owner}","repos":[{}]}}"#, repos.join(","))
}

pub fn doc(fetched_at: &str, groups: &[String]) -> String {
    format!(
        r#"{{"fetched_at":"{fetched_at}","groups":[{}]}}"#,
        groups.join(",")
    )
}

pub fn mnemra_repos() -> Vec<String> {
    vec![repo("fixture-a", "first"), repo("fixture-b", "")]
}

pub fn prmanahan_repos() -> Vec<String> {
    vec![
        repo("fixture-c", "c"),
        repo("fixture-d", "d"),
        repo("fixture-e", "e"),
        repo("fixture-f", "f"),
        repo("fixture-g", ""),
        repo("fixture-h", "h"),
    ]
}

/// Owner counts of the default valid file.
pub const DEFAULT_COUNTS: [(&str, usize); 2] = [("mnemra", 2), ("prmanahan", 6)];

/// A valid file with the given `fetched_at` and the default repos.
pub fn valid_doc(fetched_at: &str) -> String {
    doc(
        fetched_at,
        &[
            group("mnemra", &mnemra_repos()),
            group("prmanahan", &prmanahan_repos()),
        ],
    )
}

/// A valid fresh file with the given repos per owner.
pub fn valid_doc_with(mnemra: &[String], prmanahan: &[String]) -> String {
    doc(
        &fetched_at(0),
        &[group("mnemra", mnemra), group("prmanahan", prmanahan)],
    )
}

/// A fresh valid file with `description_body` (raw JSON string body) on the
/// first mnemra repo.
pub fn valid_doc_with_description(description_body: &str) -> String {
    let mut m = mnemra_repos();
    m[0] = repo("fixture-a", description_body);
    valid_doc_with(&m, &prmanahan_repos())
}

/// A fresh valid file with `name` on the first mnemra repo.
pub fn valid_doc_with_name(name: &str) -> String {
    let mut m = mnemra_repos();
    m[0] = repo(name, "first");
    valid_doc_with(&m, &prmanahan_repos())
}

/// Replace `from` with `to` in `text`, panicking when `from` is absent so a
/// seed can never silently become a no-op (and so a valid file).
pub fn swap(text: &str, from: &str, to: &str) -> String {
    assert!(text.contains(from), "seed target {from:?} not found");
    text.replacen(from, to, 1)
}

/// The JSON string-escape form of one code point: `\uXXXX`, or a surrogate
/// pair for an astral code point. Built from the number so the source holds
/// no escape sequence for an editor or tool to rewrite.
pub fn json_escape_cp(cp: u32) -> String {
    let c = char::from_u32(cp).expect("a scalar value");
    let mut buf = [0u16; 2];
    c.encode_utf16(&mut buf)
        .iter()
        .map(|u| format!("{}u{:04x}", '\\', u))
        .collect()
}
