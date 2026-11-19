"""Edge-test harness: starts the BUILT `folio` binary as a process, talks to it over HTTP.

Reads nothing from the product's source and imports none of it.

What was read to build this, and why (spec gap G1, lead ruling):
  * `folio --help` and the binary's start-up panic text, to learn the subcommand (`serve`) and that
    EDGE_AUTH_TOKEN ("folio_edge_" + 64 hex) and ADMIN_PASSWORD are required.
  * `folio check-projects` error text, to learn that $STATIC_DIR/projects.json must be fresh
    (fetched_at within 30 minutes). The harness therefore writes a list file whose body is a copy of the
    committed fixture frontend/scripts/projects-fixtures/list-expected.json (read as data) with
    fetched_at set to now.
  * One grep of libs/site-core/config.rs for the names of the port and database settings (PORT,
    DATABASE_URL); the help and error text do not spell them. Spec gap: name the start-up environment.
  * frontend/src/lib/admin-api.ts, for the admin login/Bearer shape and the dashboard route
    (GET /api/admin/dashboard), per lead ruling G4.

Credentials are generated per run; none is a literal.
"""
import datetime
import http.client
import json
import os
import secrets
import shutil
import socket
import subprocess
import tempfile
import time
import unittest

BIN_VAR = "FOLIO_BIN"
FIXTURE_LIST = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "frontend", "scripts", "projects-fixtures", "list-expected.json",
)
EDGE_HEADER = "X-Folio-Edge-Auth"


def folio_bin():
    """Return the binary path, or FAIL (never skip) naming FOLIO_BIN."""
    path = os.environ.get(BIN_VAR)
    if not path:
        raise AssertionError(f"{BIN_VAR} is not set: point it at the built `folio` binary")
    if not os.path.isfile(path) or not os.access(path, os.X_OK):
        raise AssertionError(f"{BIN_VAR}={path!r} is missing or not executable")
    probe = subprocess.run([path, "--help"], capture_output=True, timeout=30)
    if probe.returncode != 0:
        raise AssertionError(f"{BIN_VAR}={path!r} did not answer `--help` with exit 0 (got {probe.returncode})")
    return path


def _free_port():
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


class Response:
    def __init__(self, status, headers, body):
        self.status = status
        self.headers = headers
        self.body = body

    def header(self, name):
        return self.headers.get(name.lower())


class FolioServer:
    """One running binary with its own temp static dir, database, port and credentials."""

    def __init__(self, binary):
        self.binary = binary
        self.dir = tempfile.mkdtemp(prefix="folio-edge-")
        self.static_dir = os.path.join(self.dir, "static")
        os.mkdir(self.static_dir)
        with open(os.path.join(self.static_dir, "index.html"), "w") as f:
            f.write("<!doctype html><title>edge</title>")
        with open(FIXTURE_LIST) as f:
            projects = json.load(f)
        projects["fetched_at"] = datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")
        with open(os.path.join(self.static_dir, "projects.json"), "w") as f:
            json.dump(projects, f)
        self.admin_password = secrets.token_urlsafe(24)
        self.edge_token = "folio_edge_" + secrets.token_hex(32)
        self.port = _free_port()
        self.log_path = os.path.join(self.dir, "server.log")
        self._log = open(self.log_path, "wb")
        env = {
            "PATH": os.environ.get("PATH", ""),
            "STATIC_DIR": self.static_dir,
            "DATABASE_URL": os.path.join(self.dir, "site.db"),
            "ADMIN_PASSWORD": self.admin_password,
            "EDGE_AUTH_TOKEN": self.edge_token,
            "PORT": str(self.port),
        }
        self.proc = subprocess.Popen([binary, "serve"], env=env, stdout=self._log, stderr=subprocess.STDOUT)
        self._wait_ready()

    def _wait_ready(self, timeout=60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.proc.poll() is not None:
                raise AssertionError(f"folio exited early (code {self.proc.returncode}):\n{self.log_text()}")
            try:
                if self.request("GET", "/api/health", edge=True).status == 200:
                    return
            except OSError:
                pass
            time.sleep(0.05)
        raise AssertionError(f"folio not ready within {timeout}s:\n{self.log_text()}")

    def log_text(self):
        self._log.flush()
        with open(self.log_path, "r", errors="replace") as f:
            return f.read()

    def request(self, method, path, edge=False, headers=None, body=None):
        """Raw HTTP: `path` is sent exactly as given (no client-side normalisation or encoding)."""
        hdrs = dict(headers or {})
        if edge:
            hdrs[EDGE_HEADER] = self.edge_token
        conn = http.client.HTTPConnection("127.0.0.1", self.port, timeout=20)
        try:
            conn.putrequest(method, path, skip_accept_encoding=True)
            data = None
            if body is not None:
                data = json.dumps(body).encode()
                hdrs["Content-Type"] = "application/json"
            if data is not None or method in ("POST", "PUT", "PATCH"):
                hdrs["Content-Length"] = str(len(data or b""))
            for k, v in hdrs.items():
                conn.putheader(k, v)
            conn.endheaders(data)
            resp = conn.getresponse()
            return Response(resp.status, {k.lower(): v for k, v in resp.getheaders()}, resp.read())
        finally:
            conn.close()

    def admin_token(self):
        resp = self.request("POST", "/api/admin/login", edge=True, body={"password": self.admin_password})
        if resp.status != 200:
            raise AssertionError(f"admin login failed: {resp.status} {resp.body!r}")
        return json.loads(resp.body)["token"]

    def stop(self):
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        self._log.close()
        shutil.rmtree(self.dir, ignore_errors=True)


class EdgeCase(unittest.TestCase):
    """Fresh server per test; fails (not skips) when the binary is absent."""

    def setUp(self):
        self.server = FolioServer(folio_bin())
        self.addCleanup(self.server.stop)
