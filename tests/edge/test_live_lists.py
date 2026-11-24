"""Edge tests for merge 2 of #4853, the live lists. Black box: only the built `folio` binary, as a process and over HTTP.

Test plan: docs/test-plans/2026-11-18-live-lists-test-plan.md (ids LL-PY-1..18; the Playwright half, LL-PW-1..5,
lives in e2e/projects.spec.ts). Spec: docs/specs/2026-11-18-live-lists-and-blog-move.md.
Start command, environment, the seeding method and the log format: see the header of harness.py (what was read, why).

The binary has no source override, so during a run the refresher may reach the real GitHub and mnemra.dev or fail.
Every assertion below holds in both worlds. Routing areas (LL-PY-5..8, 14..16) set LIVE_LISTS_DISABLED=projects,articles
so the lists routes answer deterministically.

Inherited pins (plan "Rate-limit pin" and G3, quoted from the plan's review of the pre-merge-2 source, which this file
did not read): limiter 60 requests per 60 s keyed on the peer IP, refusal 429 with `Retry-After: 60` and body
{"error":"Too many requests. Please wait before retrying."}; origin-lock refusal 404 with an empty body. LL-PY-5..8
confirm them against the built binary; a mismatch is a reported failure, not a silent adjustment.

Log normalisation (LL-PY-12, LL-PY-13): per harness.parse_log_lines, the leading timestamp and ANSI colour codes are
stripped and the run's temp directory is replaced by a placeholder; each error-level line is then the pair
(level, message-with-fields). Error lines every start already emits (the CSP lines, which carry per-run paths) fall
into the baseline run's set and so cancel.
"""
import json
import os
import secrets
import subprocess
import sys
import shutil
import tempfile
import time
import unittest

from harness import (
    TIME_GRAMMAR, BareCase, EdgeCase, folio_bin, parse_log_lines, parse_stamp,
    seed_documents, stamp,
)

UNAVAILABLE = {"error": "Projects list unavailable"}
LIMIT_BODY = {"error": "Too many requests. Please wait before retrying."}
LIMIT = 60
BOTH_OFF = {"LIVE_LISTS_DISABLED": "projects,articles"}


# ---- shared assertions -----------------------------------------------------------------------------------------

def check_projects_unavailable(t, r):
    """The LL-PY-2 (b) contract."""
    t.assertEqual(r.status, 503)
    t.assertEqual(r.header("cache-control"), "no-store")
    t.assertEqual(r.header("content-type"), "application/json")
    t.assertEqual(json.loads(r.body), UNAVAILABLE)


def check_projects_document(t, r):
    """The LL-PY-2 (a) contract; returns the parsed document."""
    t.assertEqual(r.status, 200)
    t.assertEqual(r.header("cache-control"), "no-store")
    t.assertEqual(r.header("content-type"), "application/json")
    doc = json.loads(r.body)
    t.assertEqual(set(doc), {"fetched_at", "groups"})
    t.assertRegex(doc["fetched_at"], TIME_GRAMMAR)
    t.assertIsInstance(doc["groups"], list)
    for g in doc["groups"]:
        t.assertEqual(set(g), {"owner", "repos"})
        t.assertIsInstance(g["owner"], str)
        t.assertTrue(g["owner"])
        t.assertIsInstance(g["repos"], list)
        for repo in g["repos"]:
            t.assertEqual(set(repo), {"name", "description"})
            t.assertIsInstance(repo["name"], str)
            t.assertIsInstance(repo["description"], str)
    return doc


def check_projects_either(t, r):
    if r.status == 200:
        return check_projects_document(t, r)
    check_projects_unavailable(t, r)
    return None


def check_freshness(t, r):
    """The LL-PY-3 contract; returns the parsed object."""
    t.assertEqual(r.status, 200)
    t.assertEqual(r.header("cache-control"), "no-store")
    body = json.loads(r.body)
    t.assertIsInstance(body, dict)
    t.assertEqual(set(body), {"projects", "articles", "started_at"})
    t.assertIsInstance(body["started_at"], str)
    t.assertRegex(body["started_at"], TIME_GRAMMAR)
    for k in ("projects", "articles"):
        if body[k] is not None:
            t.assertIsInstance(body[k], str)
            t.assertRegex(body[k], TIME_GRAMMAR)
    return body


def check_refusal_429(t, r):
    t.assertEqual(r.status, 429)
    t.assertEqual(r.header("retry-after"), "60")
    t.assertEqual(json.loads(r.body), LIMIT_BODY)


def error_lines(server):
    server.stop_process()
    return {line for line in parse_log_lines(server.log_text(), server.dir) if line[0] == "ERROR"}


def burst_until_refused(t, server, route_path, check_first):
    """Plan "Rate-limit pin", steps 1-5.

    Arithmetic: request 1 is the read of the route under test (counted: it is behind the limiter), requests 2..60
    are 59 reads of /api/profile, request 61 is one more /api/profile read, which must be the refusal; then the
    route under test again, which must be the same refusal.
    """
    t0 = time.monotonic()
    first = server.request("GET", route_path, edge=True)
    check_first(t, first)
    counted = 1
    while counted < LIMIT:
        r = server.request("GET", "/api/profile", edge=True)
        counted += 1
        t.assertNotEqual(r.status, 429, f"refused at request {counted}, before the limit of {LIMIT} was spent")
    t.assertEqual(counted, LIMIT)
    over = server.request("GET", "/api/profile", edge=True)  # request 61
    elapsed = time.monotonic() - t0
    t.assertLess(elapsed, 55, f"INCONCLUSIVE: 61 requests took {elapsed:.1f}s, too slow for the 60 s window")
    check_refusal_429(t, over)
    check_refusal_429(t, server.request("GET", route_path, edge=True))


# ---- start-up -----------------------------------------------------------------------------------------------------

class TestStart(BareCase):
    def test_LL_PY_1_starts_with_no_list_file_and_health_answers_within_15s(self):
        # R-29 (last bullet), R-5, R-16. Given a STATIC_DIR with index.html and no projects.json, a fresh database
        # and LIVE_LISTS_DISABLED unset, when the binary is spawned, then health answers 200 within 15 s and the
        # process is still running.
        server = self.start(ready_timeout=15)
        self.assertIsNone(server.proc.poll(), "process exited after answering health")
        self.assertFalse(os.path.exists(os.path.join(server.static_dir, "projects.json")))

    def test_LL_PY_17_starts_with_a_list_file_that_fails_the_projects_validator(self):
        # R-29 ("or invalid" clause). Given a projects.json of the right shape whose fetched_at is "2026" (outside
        # the grammar), the variable unset, when the binary is spawned, then health answers 200 within 15 s and the
        # process is still running. (This run picks the fetched_at variant, not the U+202E one.)
        projects, _ = seed_documents()
        projects["fetched_at"] = "2026"
        server = self.start(static_files={"projects.json": projects}, ready_timeout=15)
        self.assertIsNone(server.proc.poll())

    def test_LL_PY_18_starts_with_a_malformed_list_file(self):
        # R-29. Given a projects.json that is not valid JSON, the variable unset, when the binary is spawned, then
        # health answers 200 within 15 s and the process is still running.
        server = self.start(static_files={"projects.json": b"{not json"}, ready_timeout=15)
        self.assertIsNone(server.proc.poll())


# ---- the two routes ---------------------------------------------------------------------------------------------

class TestRoutes(EdgeCase):
    def test_LL_PY_2_projects_route_answers_a_document_or_the_unavailable_error(self):
        # R-21, API Contract. Given a started server and the valid token, when GET /api/lists/projects, then
        # either the document (200) or 503 {"error":"Projects list unavailable"}, no-store and JSON either way.
        check_projects_either(self, self.server.request("GET", "/api/lists/projects", edge=True))

    def test_LL_PY_3_freshness_route_has_exactly_its_three_keys(self):
        # R-22, API Contract. Given a started server and the valid token, when GET /api/lists/freshness, then 200,
        # no-store, exactly {projects, articles, started_at}, each time in the fetched_at grammar or null.
        check_freshness(self, self.server.request("GET", "/api/lists/freshness", edge=True))


class TestStartedAt(BareCase):
    def test_LL_PY_4_started_at_is_fixed_before_the_first_request(self):
        # R-22. Given instant A recorded immediately before spawn, when health first answers 200 and instant B is
        # recorded, then started_at lies in [A, B] and two reads return the same value. A is floored to the
        # millisecond because started_at is expressed in milliseconds (a representation match, not a tolerance).
        server = self.start()
        a = parse_stamp(stamp(server.spawned_at))
        b = server.ready_at
        first = check_freshness(self, server.request("GET", "/api/lists/freshness", edge=True))
        second = check_freshness(self, server.request("GET", "/api/lists/freshness", edge=True))
        started = parse_stamp(first["started_at"])
        self.assertGreaterEqual(started, a)
        self.assertLessEqual(started, b)
        self.assertEqual(first["started_at"], second["started_at"])


# ---- origin lock and limiter ----------------------------------------------------------------------------------

class TestOriginLockAndLimiter(BareCase):
    def _locked(self, path, check_control):
        server = self.start(env=BOTH_OFF)
        r = server.request("GET", path)
        other = server.request("GET", "/unrelated-" + secrets.token_hex(4))
        self.assertEqual(r.status, 404)
        self.assertEqual(r.body, b"")
        self.assertEqual((r.status, r.body, r.header("content-type")),
                         (other.status, other.body, other.header("content-type")))
        # Control: the same route with the valid header is the route's own answer, so the 404 above is the lock.
        check_control(self, server.request("GET", path, edge=True))

    def test_LL_PY_5_projects_route_without_the_edge_token_gets_the_origin_lock_refusal(self):
        # R-23, First run. Given an edge token set and both lists disabled, when /api/lists/projects is requested
        # without the header, then 404 with an empty body, equal to an unrelated path's refusal.
        self._locked("/api/lists/projects", check_projects_unavailable)

    def test_LL_PY_6_freshness_route_without_the_edge_token_gets_the_origin_lock_refusal(self):
        # R-23, First run. Same as LL-PY-5 for /api/lists/freshness.
        self._locked("/api/lists/freshness", lambda t, r: check_freshness(t, r))

    def test_LL_PY_7_limiter_refuses_the_projects_route_once_the_limit_is_spent(self):
        # R-23. Given a fresh server (both lists disabled, valid token, no Bearer), when 60 limited requests have
        # been made and one more, then request 61 and the projects route are the pinned 429.
        server = self.start(env=BOTH_OFF)
        burst_until_refused(self, server, "/api/lists/projects", check_projects_unavailable)

    def test_LL_PY_8_limiter_refuses_the_freshness_route_once_the_limit_is_spent(self):
        # R-23. Same as LL-PY-7 for /api/lists/freshness, on its own fresh server.
        server = self.start(env=BOTH_OFF)
        burst_until_refused(self, server, "/api/lists/freshness", lambda t, r: check_freshness(t, r))


# ---- the switch -----------------------------------------------------------------------------------------------

class TestSwitch(BareCase):
    def _start_seeded(self, env):
        projects, articles = seed_documents()
        server = self.start(env=env, seed=(projects, articles))
        return server, projects, articles

    def _freshness(self, server):
        return check_freshness(self, server.request("GET", "/api/lists/freshness", edge=True))

    def test_LL_PY_9_switch_projects_clears_the_projects_list_and_keeps_articles(self):
        # R-18, R-19. Given seeded rows and LIVE_LISTS_DISABLED=projects, when the server starts and is read, then
        # the projects route is the 503 contract, freshness projects is null and articles is not, and after stop
        # the store has no projects row and still has the articles row; a log line names projects and `deleted`.
        server, _, _ = self._start_seeded({"LIVE_LISTS_DISABLED": "projects"})
        check_projects_unavailable(self, server.request("GET", "/api/lists/projects", edge=True))
        fresh = self._freshness(server)
        self.assertIsNone(fresh["projects"])
        self.assertIsNotNone(fresh["articles"])
        server.stop_process()
        rows = server.live_list_rows()
        self.assertNotIn("projects", rows)
        self.assertIn("articles", rows)
        messages = [m for _, m in parse_log_lines(server.log_text(), server.dir)]
        self.assertTrue(any("projects" in m and "deleted" in m for m in messages), "\n".join(messages))

    def test_LL_PY_10_switch_articles_clears_only_the_articles_list(self):
        # R-18, R-19. Given seeded rows and LIVE_LISTS_DISABLED=articles, when the server starts and is read, then
        # freshness articles is null, the projects route is the full 200 contract (equal to the seeded document if
        # its fetched_at is the seeded one), freshness projects is not null, after stop the store has no articles
        # row and still a projects row, and a log line names articles and `deleted`.
        server, projects, _ = self._start_seeded({"LIVE_LISTS_DISABLED": "articles"})
        doc = check_projects_document(self, server.request("GET", "/api/lists/projects", edge=True))
        if doc["fetched_at"] == projects["fetched_at"]:
            self.assertEqual(doc, projects)
        fresh = self._freshness(server)
        self.assertIsNone(fresh["articles"])
        self.assertIsNotNone(fresh["projects"])
        server.stop_process()
        rows = server.live_list_rows()
        self.assertNotIn("articles", rows)
        self.assertIn("projects", rows)
        messages = [m for _, m in parse_log_lines(server.log_text(), server.dir)]
        self.assertTrue(any("articles" in m and "deleted" in m for m in messages), "\n".join(messages))

    def test_LL_PY_11_switch_absent_serves_the_held_copies_from_the_first_request(self):
        # R-18 (absent), R-16. Given seeded rows and the variable unset, when the server starts and the first
        # request after health is /api/lists/projects, then the full 200 contract (equal to the seeded document if
        # its fetched_at is the seeded one), both freshness times non-null, and after stop both rows still exist.
        server, projects, _ = self._start_seeded({})
        doc = check_projects_document(self, server.request("GET", "/api/lists/projects", edge=True))
        if doc["fetched_at"] == projects["fetched_at"]:
            self.assertEqual(doc, projects)
        fresh = self._freshness(server)
        self.assertIsNotNone(fresh["projects"])
        self.assertIsNotNone(fresh["articles"])
        server.stop_process()
        self.assertEqual(set(server.live_list_rows()), {"projects", "articles"})

    def _unclean_switch_disables_both(self, value, token_name):
        server, _, _ = self._start_seeded({"LIVE_LISTS_DISABLED": value})
        baseline, _, _ = self._start_seeded({})
        check_projects_unavailable(self, server.request("GET", "/api/lists/projects", edge=True))
        fresh = self._freshness(server)
        self.assertIsNone(fresh["projects"])
        self.assertIsNone(fresh["articles"])
        self.assertEqual(server.request("GET", "/api/health", edge=True).status, 200)
        errors = error_lines(server)
        base_errors = error_lines(baseline)
        self.assertEqual(server.live_list_rows(), {})
        new = errors - base_errors
        self.assertTrue(new, f"no error-level line beyond the baseline's; switched run errors: {sorted(errors)}")
        names_it = any("LIVE_LISTS_DISABLED" in m or token_name in m for _, m in new)
        print(f"[observation] {token_name!r} run: new error lines {sorted(new)}; "
              f"names the switch or token: {names_it}", file=sys.stderr)

    def test_LL_PY_12_unknown_token_disables_both_lists_and_logs_an_error_yet_the_server_starts(self):
        # R-18 (unknown token), R-19. Given seeded rows and LIVE_LISTS_DISABLED=articles,projcts, when the server
        # starts, then the projects route is the 503 contract, both freshness times null, health 200, the store has
        # no rows after stop, and the switched run has an error-level (level, message) the baseline run lacks.
        self._unclean_switch_disables_both("articles,projcts", "projcts")

    def test_LL_PY_13_a_non_unicode_value_is_read_as_set_not_as_absent(self):
        # R-18 (non-Unicode), R-5. Given seeded rows and a child environment holding LIVE_LISTS_DISABLED as the
        # bytes b"articles\xff" (set through the Popen env mapping, not os.environb), when the server starts, then
        # the same observations as LL-PY-12. A lossy read would look like "nothing disabled".
        self._unclean_switch_disables_both(b"articles\xff", "non-unicode")


# ---- retired surfaces ---------------------------------------------------------------------------------------------

class TestRetired(BareCase):
    def test_LL_PY_14_check_projects_subcommand_is_gone(self):
        # R-29 (subcommand). Given a STATIC_DIR with a valid, fresh projects.json (the shape on which the retired
        # subcommand exited 0), when `folio check-projects` runs, then it exits non-zero and does not serve.
        projects, _ = seed_documents()
        d = tempfile.mkdtemp(prefix="folio-edge-cp-")
        self.addCleanup(lambda: shutil.rmtree(d, ignore_errors=True))
        os.mkdir(os.path.join(d, "static"))
        with open(os.path.join(d, "static", "index.html"), "w") as f:
            f.write("<!doctype html><title>edge</title>")
        with open(os.path.join(d, "static", "projects.json"), "w") as f:
            json.dump(projects, f)
        env = {
            "PATH": os.environ.get("PATH", ""),
            "STATIC_DIR": os.path.join(d, "static"),
            "DATABASE_URL": os.path.join(d, "site.db"),
            "ADMIN_PASSWORD": secrets.token_urlsafe(24),
            "EDGE_AUTH_TOKEN": "folio_edge_" + secrets.token_hex(32),
            "PORT": "0",
        }
        try:
            done = subprocess.run([folio_bin(), "check-projects"], env=env, capture_output=True, timeout=30)
        except subprocess.TimeoutExpired:
            self.fail("`folio check-projects` did not exit within 30 s (it started serving?)")
        self.assertNotEqual(done.returncode, 0)

    def test_LL_PY_15_projects_json_has_no_special_cache_rule_when_the_file_exists(self):
        # R-29. Given a STATIC_DIR with projects.json and a control.json of the same body, both lists disabled,
        # when both are read, then status, Content-Type and Cache-Control equal the control's, and none says no-cache.
        projects, _ = seed_documents()
        server = self.start(env=BOTH_OFF, static_files={"projects.json": projects, "control.json": projects})
        got = server.request("GET", "/projects.json", edge=True)
        control = server.request("GET", "/control.json", edge=True)
        for h in ("content-type", "cache-control"):
            self.assertEqual(got.header(h), control.header(h), h)
        self.assertEqual(got.status, control.status)
        self.assertNotIn("no-cache", got.header("cache-control") or "")

    def test_LL_PY_16_projects_json_has_no_special_cache_rule_when_the_file_is_absent(self):
        # R-29. Given a STATIC_DIR with no projects.json, both lists disabled, when /projects.json and a random
        # nonexistent .json are read, then status, Content-Type, Cache-Control and body are equal, with no no-cache.
        server = self.start(env=BOTH_OFF)
        got = server.request("GET", "/projects.json", edge=True)
        control = server.request("GET", "/control-" + secrets.token_hex(4) + ".json", edge=True)
        self.assertEqual(got.status, control.status)
        for h in ("content-type", "cache-control"):
            self.assertEqual(got.header(h), control.header(h), h)
        self.assertEqual(got.body, control.body)
        self.assertNotIn("no-cache", got.header("cache-control") or "")
        self.assertNotIn("no-cache", control.header("cache-control") or "")


if __name__ == "__main__":
    unittest.main()
