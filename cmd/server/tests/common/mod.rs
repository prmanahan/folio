//! Spawned-binary harness for `folio serve` (`serve_lists.rs`, and `server.rs` beside it).
//!
//! Black-box: every observation crosses the process boundary (exit code,
//! combined stdout/stderr, or what the running server answers over HTTP).
//! Nothing here links the crate under test.
//!
//! Isolation, per R-18: the child runs from an empty temp cwd outside the
//! checkout (no `.env` reachable), with its environment cleared and then
//! given only an absolute STATIC_DIR, a DATABASE_URL naming a file in an
//! empty temp dir, and a PORT this process holds open on 0.0.0.0. The test
//! process mutates no process-global state: env goes to the child only.
#![allow(dead_code)]

pub mod server;

use std::fs;
use std::io::Read;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

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

    /// Run `folio serve` the way production does, minus the real secrets:
    /// a fake ADMIN_PASSWORD, a well-formed fake EDGE_AUTH_TOKEN, and a
    /// STATIC_DIR holding an `index.html` with a CSP meta tag. Those three
    /// keep an exit from coming from the ADMIN_PASSWORD panic, the
    /// EDGE_AUTH_TOKEN panic or `validate_static_dir`. RUST_LOG is left
    /// unset (the environment is cleared). The port is held by this
    /// process on 0.0.0.0, so a server that gets as far as binding dies
    /// there instead of serving: nothing here ever needs killing.
    pub fn run_serve(&self) -> Outcome {
        fs::write(
            self.static_dir.join("index.html"),
            format!(
                "<!doctype html><html><head><meta http-equiv=\"{}\" content=\"default-src 'self'\"></head><body></body></html>",
                "content-security-policy"
            ),
        )
        .unwrap();
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_folio"));
        cmd.arg("serve")
            .current_dir(&self.cwd)
            .env_clear()
            .env("ADMIN_PASSWORD", fake_admin_password())
            .env("EDGE_AUTH_TOKEN", fake_edge_auth_token())
            .env("STATIC_DIR", &self.static_dir)
            .env("DATABASE_URL", &self.db_path)
            .env("PORT", self.port().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        collect(cmd)
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

/// Spawn `cmd`, collect combined output, and kill it at the deadline.
fn collect(mut cmd: Command) -> Outcome {
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

/// What a server that reached `TcpListener::bind` on the held port reports.
pub const BIND_FAILURE: &str = "Failed to bind to port";

/// Fake, never a real credential. Built at run time so no secret literal
/// sits in the source.
pub fn fake_admin_password() -> String {
    format!("fake-admin-{}", "x".repeat(16))
}

/// `folio_edge_` plus 64 hex characters: the shape `Config` accepts.
pub fn fake_edge_auth_token() -> String {
    format!("folio_edge_{}", "ab".repeat(32))
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
