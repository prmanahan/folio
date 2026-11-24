"""Edge-test harness: starts the BUILT `folio` binary as a process, talks to it over HTTP.

Reads nothing from the product's source and imports none of it.

What was read to build this, and why (spec gap G1, lead ruling):
  * `folio --help` and the binary's start-up panic text, to learn the subcommand (`serve`) and that
    EDGE_AUTH_TOKEN ("folio_edge_" + 64 hex) and ADMIN_PASSWORD are required.
  * One grep of libs/site-core/config.rs for the names of the port and database settings (PORT,
    DATABASE_URL); the help and error text do not spell them. Spec gap: name the start-up environment.
  * frontend/src/lib/admin-api.ts, for the admin login/Bearer shape and the dashboard route
    (GET /api/admin/dashboard), per lead ruling G4.
  * frontend/scripts/projects-fixtures/list-expected.json, read as data: the projects document the harness seeds.

Merge 2 (live lists) additions, T17 of #4853:
  * `folio check-projects` is now an unknown clap subcommand; read from `folio --help` and one run.
  * One real run's stdout, to learn the log line format: `<ISO timestamp> <LEVEL> <target>: <message> key=value ...`,
    wrapped in ANSI colour codes. `parse_log_lines` strips the timestamp and the ANSI codes and reduces a line to
    (level, message-with-fields) with the run's directory replaced by a placeholder. Nothing else of the source
    was read.
  * `live_lists(name, document)` table, from the spec's Data Model (the plan's Fixtures section); the harness seeds
    rows with stdlib `sqlite3` after a first start of the binary has run the migration, then starts the binary under
    test over that database file.
  * The harness no longer writes a `projects.json` by default: the binary does not read one any more (R-29). A test
    that wants one passes it in `static_files`.

Credentials are generated per run; none is a literal.
"""
import atexit
import datetime
import http.client
import json
import os
import re
import secrets
import shutil
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest

BIN_VAR = "FOLIO_BIN"
FIXTURE_LIST = os.path.join(
    os.path.dirname(os.path.abspath(__file__)),
    "..", "..", "frontend", "scripts", "projects-fixtures", "list-expected.json",
)
EDGE_HEADER = "X-Folio-Edge-Auth"

TIME_GRAMMAR = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$")
LIST_NAMES = ("projects", "articles")
_ANSI = re.compile(r"\x1b\[[0-9;]*m")
_LOG_LINE = re.compile(r"^\S+\s+(ERROR|WARN|INFO|DEBUG|TRACE)\s+(.*)$")

# Starts that left a list enabled, for the outbound-budget count the plan asks for (about 2 GitHub requests per
# start with projects enabled, 1 mnemra.dev request per start with articles enabled). Printed once at exit.
START_COUNT = {"projects": 0, "articles": 0, "all": 0}


@atexit.register
def _report_starts():
    print(f"[edge harness] binary starts (excluding seeding starts): {START_COUNT['all']}; "
          f"with projects enabled: {START_COUNT['projects']}; with articles enabled: {START_COUNT['articles']}",
          file=sys.stderr)


def _enabled_lists(value):
    """Which lists a LIVE_LISTS_DISABLED value leaves enabled (a model of R-18, used only for the budget count)."""
    if value is None:
        return set(LIST_NAMES)
    if isinstance(value, bytes):
        try:
            value = value.decode("utf-8")
        except UnicodeDecodeError:
            return set()
    tokens = [t.strip() for t in value.split(",")]
    if any(t and t not in LIST_NAMES for t in tokens):
        return set()
    return set(LIST_NAMES) - set(tokens)


def stamp(dt=None):
    """A time in the `fetched_at` grammar (YYYY-MM-DDTHH:MM:SS.sssZ)."""
    dt = dt or datetime.datetime.now(datetime.timezone.utc)
    return dt.strftime("%Y-%m-%dT%H:%M:%S.") + "%03dZ" % (dt.microsecond // 1000)


def parse_stamp(text):
    return datetime.datetime.strptime(text, "%Y-%m-%dT%H:%M:%S.%fZ").replace(tzinfo=datetime.timezone.utc)


def seed_documents():
    """A valid projects document and a valid articles document (spec Data Model), `fetched_at` now."""
    with open(FIXTURE_LIST) as f:
        projects = json.load(f)
    now = stamp()
    projects["fetched_at"] = now
    slug = "edge-" + secrets.token_hex(4)
    articles = {
        "fetched_at": now,
        "posts": [{
            "title": "Edge seeded post " + slug,
            "summary": "A post written by the edge harness.",
            "link": "https://mnemra.dev/blog/" + slug + "/",
            "published": now,
        }],
    }
    return projects, articles


def parse_log_lines(text, workdir):
    """[(level, message-with-fields)] with timestamp and ANSI codes stripped and the run's directory replaced."""
    out = []
    for raw in text.splitlines():
        line = _ANSI.sub("", raw).replace(workdir, "<DIR>")
        m = _LOG_LINE.match(line)
        if m:
            out.append((m.group(1), m.group(2)))
    return out


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
    """One running binary with its own temp static dir, database, port and credentials.

    env: extra child environment (str or bytes values; a None value leaves the variable unset).
    static_files: {name: bytes|str|dict} written into STATIC_DIR next to index.html (a dict is dumped as JSON).
    seed: (projects_doc, articles_doc) inserted into live_lists before the start under test; either may be None.
    ready_timeout: seconds to wait for /api/health 200; expect_ready False skips the wait (the caller does it).
    """

    def __init__(self, binary, env=None, static_files=None, seed=None, ready_timeout=60, expect_ready=True):
        self.binary = binary
        self.dir = tempfile.mkdtemp(prefix="folio-edge-")
        self.static_dir = os.path.join(self.dir, "static")
        os.mkdir(self.static_dir)
        with open(os.path.join(self.static_dir, "index.html"), "w") as f:
            f.write("<!doctype html><title>edge</title>")
        for name, content in (static_files or {}).items():
            if isinstance(content, dict):
                content = json.dumps(content)
            with open(os.path.join(self.static_dir, name), "wb" if isinstance(content, bytes) else "w") as f:
                f.write(content)
        self.admin_password = secrets.token_urlsafe(24)
        self.edge_token = "folio_edge_" + secrets.token_hex(32)
        self.port = _free_port()
        self.db_path = os.path.join(self.dir, "site.db")
        self.log_path = os.path.join(self.dir, "server.log")
        self.proc = None
        self._log = None
        self.spawned_at = None
        self.ready_at = None
        try:
            if seed is not None:
                self._seed(*seed)
            self._spawn(dict(env or {}))
            if expect_ready:
                self.wait_ready(ready_timeout)
        except BaseException:
            self.stop()
            raise

    def _spawn(self, extra, log_path=None, count=True):
        env = {
            "PATH": os.environ.get("PATH", ""),
            "STATIC_DIR": self.static_dir,
            "DATABASE_URL": self.db_path,
            "ADMIN_PASSWORD": self.admin_password,
            "EDGE_AUTH_TOKEN": self.edge_token,
            "PORT": str(self.port),
        }
        for k, v in extra.items():
            if v is None:
                env.pop(k, None)
            else:
                env[k] = v
        if count:
            START_COUNT["all"] += 1
            for name in _enabled_lists(env.get("LIVE_LISTS_DISABLED")):
                START_COUNT[name] += 1
        self._log = open(log_path or self.log_path, "wb")
        self.spawned_at = datetime.datetime.now(datetime.timezone.utc)
        self.proc = subprocess.Popen([self.binary, "serve"], env=env, stdout=self._log, stderr=subprocess.STDOUT)

    def _seed(self, projects, articles):
        # A first start with both lists disabled (no outbound request, no rows written) runs migration 008;
        # then the rows go in with stdlib sqlite3 and the binary under test starts over the same file.
        self._spawn({"LIVE_LISTS_DISABLED": "projects,articles"}, log_path=os.path.join(self.dir, "seed.log"), count=False)
        self.wait_ready(60)
        self.stop_process()
        con = sqlite3.connect(self.db_path)
        try:
            for name, doc in (("projects", projects), ("articles", articles)):
                if doc is not None:
                    con.execute("INSERT INTO live_lists(name, document) VALUES (?, ?)", (name, json.dumps(doc)))
            con.commit()
        finally:
            con.close()

    def wait_ready(self, timeout=60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.proc.poll() is not None:
                raise AssertionError(f"folio exited early (code {self.proc.returncode}):\n{self.log_text()}")
            try:
                if self.request("GET", "/api/health", edge=True).status == 200:
                    self.ready_at = datetime.datetime.now(datetime.timezone.utc)
                    return
            except OSError:
                pass
            time.sleep(0.05)
        raise AssertionError(f"folio not ready within {timeout}s:\n{self.log_text()}")

    def log_text(self):
        if not self._log.closed:
            self._log.flush()
        with open(self.log_path, "r", errors="replace") as f:
            return f.read()

    def live_list_rows(self):
        """{name: parsed document} read straight from the store (call after the process has stopped)."""
        con = sqlite3.connect(self.db_path)
        try:
            return {n: json.loads(d) for n, d in con.execute("SELECT name, document FROM live_lists")}
        finally:
            con.close()

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

    def stop_process(self):
        if self.proc is not None and self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait()
        if self._log is not None and not self._log.closed:
            self._log.close()

    def stop(self):
        self.stop_process()
        shutil.rmtree(self.dir, ignore_errors=True)


class EdgeCase(unittest.TestCase):
    """Fresh server per test; fails (not skips) when the binary is absent."""

    def setUp(self):
        self.server = FolioServer(folio_bin())
        self.addCleanup(self.server.stop)

    def start(self, **kwargs):
        """Start another server (own dir, port, token) with options; cleaned up with the test."""
        server = FolioServer(folio_bin(), **kwargs)
        self.addCleanup(server.stop)
        return server


class BareCase(unittest.TestCase):
    """No server started in setUp: the test builds its own with options (still fails when the binary is absent)."""

    def start(self, **kwargs):
        server = FolioServer(folio_bin(), **kwargs)
        self.addCleanup(server.stop)
        return server
