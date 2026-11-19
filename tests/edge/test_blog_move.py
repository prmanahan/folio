"""Edge tests for merge 1 of #4853, the blog move. Black box: only the built `folio` binary over HTTP.

Test plan: docs/test-plans/2026-11-18-blog-move-test-plan.md (ids BM-PY-1..7).
Spec: docs/specs/2026-11-18-live-lists-and-blog-move.md (R-23 /articles clause, R-25, R-27).
Start command, environment and the dashboard route: see the header of harness.py (what was read, and why).
The Playwright half of the plan (BM-PW-1..5) lives in e2e/.
"""
import json
import secrets
import unittest

from harness import EdgeCase

BLOG = "https://mnemra.dev/blog/"
NOT_FOUND_JSON = b'{"error":"Not found"}'


class TestRedirect(EdgeCase):
    def test_BM_PY_1_get_articles_path_redirects_permanently_to_the_blog(self):
        # R-25. Given a valid edge token, when GET /articles/x, then 301 to the blog with a 1h public cache.
        r = self.server.request("GET", "/articles/x", edge=True)
        self.assertEqual(r.status, 301)
        self.assertEqual(r.header("location"), BLOG)
        self.assertEqual(r.header("cache-control"), "public, max-age=3600")

    def test_BM_PY_2_post_articles_path_redirects_the_same_way(self):
        # R-25. Given a valid edge token, when POST /articles/x, then the same 301 and headers.
        r = self.server.request("POST", "/articles/x", edge=True, body={})
        self.assertEqual(r.status, 301)
        self.assertEqual(r.header("location"), BLOG)
        self.assertEqual(r.header("cache-control"), "public, max-age=3600")

    def test_BM_PY_3_articles_without_edge_token_gets_the_origin_lock_refusal_not_a_redirect(self):
        # R-23 (/articles clause). Given an edge token is configured, when GET /articles without it,
        # then the response equals what an unrelated path gets without it (the lock's refusal), and is not 301.
        r = self.server.request("GET", "/articles")
        other = self.server.request("GET", "/unrelated-" + secrets.token_hex(4))
        self.assertNotEqual(r.status, 301)
        self.assertIsNone(r.header("location"))
        self.assertEqual((r.status, r.body, r.header("content-type")), (other.status, other.body, other.header("content-type")))
        # Control: the refusal is not just "everything 404s": the same path with the token is the redirect.
        self.assertEqual(self.server.request("GET", "/articles", edge=True).status, 301)

    def test_BM_PY_7_undecodable_path_is_neither_a_redirect_nor_a_server_error(self):
        # R-25. Given a valid edge token, when GET /articles/%FF, then neither 301 nor 5xx.
        r = self.server.request("GET", "/articles/%FF", edge=True)
        self.assertNotEqual(r.status, 301)
        self.assertFalse(500 <= r.status <= 599, f"got {r.status}")


class TestRemovedApis(EdgeCase):
    def test_BM_PY_4_public_articles_api_is_gone(self):
        # R-27. When GET /api/articles with no session, then 404 JSON {"error":"Not found"}.
        r = self.server.request("GET", "/api/articles", edge=True)
        self.assertEqual(r.status, 404)
        self.assertEqual(r.header("content-type"), "application/json")
        self.assertEqual(r.body, NOT_FOUND_JSON)

    def test_BM_PY_5_admin_articles_api_is_gone_for_a_logged_in_admin(self):
        # R-27. Given an admin session, when GET /api/admin/articles, then 404 JSON {"error":"Not found"}.
        auth = {"Authorization": "Bearer " + self.server.admin_token()}
        # Control: an existing admin route answers 200 with this session, so the 404 below is the removal.
        self.assertEqual(self.server.request("GET", "/api/admin/profile", edge=True, headers=auth).status, 200)
        r = self.server.request("GET", "/api/admin/articles", edge=True, headers=auth)
        self.assertEqual(r.status, 404)
        self.assertEqual(r.header("content-type"), "application/json")
        self.assertEqual(r.body, NOT_FOUND_JSON)

    def test_BM_PY_6_admin_dashboard_has_no_articles_key(self):
        # R-27. Given an admin session, when GET /api/admin/dashboard (route taken from the admin UI's own
        # call, gap G4), then 200 JSON with no `articles` key.
        auth = {"Authorization": "Bearer " + self.server.admin_token()}
        r = self.server.request("GET", "/api/admin/dashboard", edge=True, headers=auth)
        self.assertEqual(r.status, 200)
        counts = json.loads(r.body)
        self.assertIsInstance(counts, dict)
        # Control: the object is the real counts payload, not an empty one.
        self.assertIn("experiences", counts)
        self.assertNotIn("articles", counts)


if __name__ == "__main__":
    unittest.main()
