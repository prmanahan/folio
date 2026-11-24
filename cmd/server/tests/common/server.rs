//! A running `folio serve` for the spawned-binary cases of R-5.
//!
//! Black-box like the rest of `common`: the child is the built binary, and
//! every observation is an HTTP/1.1 exchange over a socket or the child's
//! stderr. `std` only: no HTTP client is added to the tree.
//!
//! The binary has no source override, so its refresher may reach the real
//! GitHub and mnemra.dev, or fail to. Callers assert only what holds either
//! way.

use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::{Sandbox, fake_admin_password, fake_edge_auth_token, fetched_at, valid_doc};

/// R-5 / fly.toml's `grace_period`.
pub const HEALTH_DEADLINE: Duration = Duration::from_secs(15);

/// A child process serving on a port, killed when dropped.
pub struct RunningServer {
    child: Child,
    port: u16,
    stderr_path: std::path::PathBuf,
    /// Time from spawn to the first 200 from `/api/health`.
    pub time_to_healthy: Duration,
}

pub struct Response {
    pub status: u16,
    /// Header names lowercased.
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Spawn `folio serve` on a free port with a valid list file, and wait for it
/// to answer health.
///
/// # Panics
/// When the child exits, or has not answered `/api/health` with 200 within
/// [`HEALTH_DEADLINE`]; the message carries the child's output.
pub fn spawn_serve(sb: &Sandbox) -> RunningServer {
    // The list file stays required until the later task that retires it.
    sb.write_list(&valid_doc(&fetched_at(0)));
    std::fs::write(
        sb.static_dir.join("index.html"),
        "<!doctype html><html><head><meta http-equiv=\"content-security-policy\" content=\"default-src 'self'\"></head><body></body></html>",
    )
    .unwrap();
    let port = {
        let probe = TcpListener::bind("127.0.0.1:0").expect("find a free port");
        probe.local_addr().unwrap().port()
    };
    let stderr_path = sb.cwd.join("child.log");
    let log = File::create(&stderr_path).expect("create the child's log");
    let child = Command::new(env!("CARGO_BIN_EXE_folio"))
        .arg("serve")
        .current_dir(&sb.cwd)
        .env_clear()
        .env("ADMIN_PASSWORD", fake_admin_password())
        .env("EDGE_AUTH_TOKEN", fake_edge_auth_token())
        .env("STATIC_DIR", &sb.static_dir)
        .env("DATABASE_URL", &sb.db_path)
        .env("PORT", port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .expect("spawn folio serve");
    let mut server = RunningServer {
        child,
        port,
        stderr_path,
        time_to_healthy: Duration::ZERO,
    };
    server.time_to_healthy = server.wait_healthy();
    server
}

impl RunningServer {
    fn wait_healthy(&mut self) -> Duration {
        let began = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("poll the child") {
                panic!(
                    "the server exited ({status}) before answering health; output:\n{}",
                    self.output()
                );
            }
            if let Ok(r) = try_get(self.port, "/api/health", &[])
                && r.status == 200
            {
                return began.elapsed();
            }
            if began.elapsed() > HEALTH_DEADLINE {
                panic!(
                    "no 200 from /api/health within {HEALTH_DEADLINE:?}; output:\n{}",
                    self.output()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// GET with the fake edge secret, so the origin lock lets it through.
    pub fn get(&self, path: &str) -> Response {
        let token = fake_edge_auth_token();
        try_get(self.port, path, &[("X-Folio-Edge-Auth", token.as_str())])
            .unwrap_or_else(|e| panic!("GET {path}: {e}; output:\n{}", self.output()))
    }

    /// The child's combined stdout and stderr so far.
    pub fn output(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        // The child may already have exited; either way it must not outlive the test.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One HTTP/1.1 GET, `Connection: close`, over `std`.
fn try_get(port: u16, path: &str, headers: &[(&str, &str)]) -> Result<Response, String> {
    let mut stream =
        TcpStream::connect(("127.0.0.1", port)).map_err(|e| format!("connect: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| format!("timeout: {e}"))?;
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n");
    for (name, value) in headers {
        request.push_str(&format!("{name}: {value}\r\n"));
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write: {e}"))?;
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .map_err(|e| format!("read: {e}"))?;
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> Result<Response, String> {
    let split = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("no end of headers")?;
    let head = String::from_utf8_lossy(&raw[..split]).into_owned();
    let mut lines = head.lines();
    let status_line = lines.next().ok_or("empty response")?;
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| format!("bad status line {status_line:?}"))?;
    let headers: Vec<(String, String)> = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(n, v)| (n.trim().to_ascii_lowercase(), v.trim().to_string()))
        .collect();
    let body_bytes = &raw[split + 4..];
    let chunked = headers
        .iter()
        .any(|(n, v)| n == "transfer-encoding" && v.eq_ignore_ascii_case("chunked"));
    let body = if chunked {
        dechunk(body_bytes)?
    } else {
        String::from_utf8_lossy(body_bytes).into_owned()
    };
    Ok(Response {
        status,
        headers,
        body,
    })
}

fn dechunk(mut rest: &[u8]) -> Result<String, String> {
    let mut out = Vec::new();
    loop {
        let eol = rest
            .windows(2)
            .position(|w| w == b"\r\n")
            .ok_or("bad chunk header")?;
        let size_text = String::from_utf8_lossy(&rest[..eol]).into_owned();
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or("").trim(), 16)
            .map_err(|e| format!("bad chunk size {size_text:?}: {e}"))?;
        rest = &rest[eol + 2..];
        if size == 0 {
            return Ok(String::from_utf8_lossy(&out).into_owned());
        }
        if rest.len() < size + 2 {
            return Err("truncated chunk".to_string());
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}
