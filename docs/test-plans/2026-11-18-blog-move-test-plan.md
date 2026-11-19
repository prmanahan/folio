# Test plan: the blog move (merge 1 of #4853)

- **Delivery:** `blog-move` (plan tasks T1 to T6; this document is T1).
- **Derived from:** `docs/specs/2026-11-18-live-lists-and-blog-move.md` (locked, blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`), the plan's D4 and D7, and folio's `CLAUDE.md` `<gates>`. Nothing else. No implementation source was read, and none is cited here. The only test code read is the grep over `e2e/` and `frontend/e2e/` in "Existing e2e specs".
- **Consumer:** T5 (edge tests). It writes one test per area below, once each. Edge cases listed under "Unit-test hand-off" belong to the implementer's unit tests, not to a second edge test.

## Requirements in scope

R-25, R-26, R-27, R-28 and R-23's `/articles` clause (spec `### Landings`, merge 1). R-29 to R-36 and R-23's two `/api/lists/` routes are merge 2 and not planned here.

## Forms

| Surface | Form | Run |
|---|---|---|
| The `folio` binary over HTTP (redirect, origin lock, removed API paths, undecodable path) | Python stdlib `unittest` + `subprocess` + `http.client`, in `tests/edge/` (plan D4) | "Run: Python suite" |
| The browser (four Articles links, admin UI absence) | Playwright TypeScript, root `e2e/`, `retries: 0` | "Run: `just e2e`" |

No third-party Python package, no Gherkin runner. Given/When/Then goes in test names and body comments.

## Areas

Each area is one way in, tested once. The ids are for T5's test names and for a union check: every id below appears in exactly one runner's test names.

### Python suite (built binary over HTTP)

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| BM-PY-1 | Redirect, one path | R-25 | Given the server running with an edge token, and a request carrying the valid `X-Folio-Edge-Auth`. When `GET /articles/x`. Then 301, `Location` exactly `https://mnemra.dev/blog/`, `Cache-Control` exactly `public, max-age=3600`. |
| BM-PY-2 | Redirect, one non-GET method | R-25 | Same, `POST /articles/x`. Then the same 301 with the same two headers. |
| BM-PY-3 | Redirect behind the origin lock | R-23 (`/articles` clause) | Given the server with an edge token set. When `GET /articles` with no `X-Folio-Edge-Auth`. Then the origin lock's refusal (First run says 404), not a 301, and no `Location` header. Assert equality with what an unrelated non-`/articles` path returns without the header, so the test does not pin wording the spec leaves open (Gap G3). |
| BM-PY-4 | Removed public API path | R-27 | When `GET /api/articles` (no session). Then 404, `Content-Type: application/json`, body exactly `{"error":"Not found"}`. |
| BM-PY-5 | Removed admin API path, with an admin session | R-27 | Given a session from `POST /api/admin/login` with the generated `ADMIN_PASSWORD`. When `GET /api/admin/articles` with that session. Then 404, `application/json`, body exactly `{"error":"Not found"}`. Control in the same test: an existing admin route answers 200 with that session, so the 404 is the removal and not a dead session. |
| BM-PY-6 | Admin dashboard response has no `articles` key | R-27 | Given an admin session. When the admin dashboard API is read. Then 200 JSON with no `articles` key. (URL unnamed in the spec: Gap G4.) |
| BM-PY-7 | Undecodable path does not 5xx | R-25 | When `GET /articles/%FF`. Then the status is neither 301 nor 500 to 599. |

R-25's other acceptance rows (forged `Host`, each `X-Forwarded-*`, query string, `/articles/`, `/articles%2Fx`, `/articlesx`, `//articles`, `HEAD`/`OPTIONS`/`PATCH`, `/articles/%zz`) are one behaviour each through an entry point already covered. They go to the unit-test hand-off, not into more edge tests.

### Playwright (`just e2e`, browser)

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| BM-PW-1 | Articles link in the nav | R-26 | Given the home page loaded with the API mocked. When the nav's Articles link is read. Then `href` exactly `https://mnemra.dev/blog/`, `target="_blank"`, `rel="noopener noreferrer"`. Assert attributes; do not click through to the off-site URL. |
| BM-PW-2 | Articles link in the hero | R-26 | Same attributes on the hero's Articles link. |
| BM-PW-3 | Articles link in the hero skeleton | R-26 | Same attributes while the hero's loading state shows (API response held back; assertion retries against a timeout, no fixed sleep). |
| BM-PW-4 | Articles link in the hub-error fallback | R-26 | Given the home page's hub API mocked to fail. When the fallback renders. Then the same attributes. |
| BM-PW-5 | Admin UI shows no articles surface | R-27 | Given a logged-in admin session (backend running; `e2e/admin.spec.ts` already logs in this way). When the dashboard and sidebar render. Then no element with `href` starting `/admin/articles`, no Articles label, no articles count, no articles quick action. |

Four link areas, not one, because R-26 names four sites and a browser reaches each by a different state (loaded, loading, error, nav), with the attribute triple mandated at each.

R-28 (`should_track`) is stated in the spec as a function result. Its only external trace is the page-hit counter's stored state, which the spec does not name a read-back for. It is a unit test; it has no edge area (Gap G5).

## Run: Python suite

The form is from plan D4 (the spec's Constraints names none: Gap G6). Prerequisite: `uv` only.

```sh
uv python install --no-config --no-bin 3.12
PY="$(uv python find --no-config --system --managed-python '>=3.12')"
FOLIO_BIN=/absolute/path/to/built/folio \
  uv run --no-config --no-project --python "$PY" -m unittest discover -v -s tests/edge -t tests/edge
```

- `FOLIO_BIN` is T5's choice of variable name; the spec names none. The harness fails, not skips, when it is unset, missing, not executable, or does not answer a liveness probe, and the failure names the variable. A run exiting 5 (no tests ran) is a failure.
- The harness writes a fresh temp `STATIC_DIR`, a temp `DATABASE_URL`, a list file (merge 1 still needs one, per the plan), and per-run generated `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`, on a free port. No credential is committed.
- The command, the binary's commit and the result are recorded in merge 1's report (D4).

## Run: `just e2e`

```sh
just e2e        # npx playwright test, from the repository root
```

- It runs `playwright.config.ts` over `e2e/` on chromium, firefox and webkit. Playwright starts Vite on :5173 itself (`webServer`); Vite proxies `/api` to the backend on :3000. Per `CLAUDE.md` `<gates>`, :8080 is the Fly/Docker port, not the e2e target.
- BM-PW-1 to BM-PW-4 run against mocked API routes and need no backend. BM-PW-5 needs the backend on :3000 started with the generated `ADMIN_PASSWORD`, as `e2e/admin.spec.ts` does.
- One spec: `npx playwright test e2e/<file>`. Playwright runs in no CI job, so the run is recorded in merge 1's report (R-26, D7).
- `npm run test:e2e` from `frontend/` is a separate suite (`frontend/e2e/ai-pane.spec.ts`, chromium). It has no `article` hit (below), so it is not part of this plan's run.
- The redirect cannot be a Playwright area: Vite on :5173 does not serve the Rust server's redirect. It is Python only.

## Existing e2e specs that visit `/articles` or assert a link to it today

Range: `grep -rniE 'article' e2e frontend/e2e`, case-insensitive over both suites, so a renamed testid or label also shows. `e2e/admin.spec.ts`, `e2e/ai-chat.spec.ts`, `e2e/hero-overflow.spec.ts`, `e2e/projects.spec.ts`, `e2e/resume.spec.ts`, `e2e/fixtures.ts` and `frontend/e2e/ai-pane.spec.ts` returned no line. Positive control: the grep returned the lines below.

| File:line | What it does today | Disposition (T5) |
|---|---|---|
| `e2e/public-pages.spec.ts:65-71` | `Articles card links to /articles`: clicks `[data-testid="card-articles"]`, expects URL `/articles`. Visits `/articles`. | Replace with the BM-PW-1 to BM-PW-4 attribute assertions; remove the click-through. |
| `e2e/public-pages.spec.ts:165-166` | `articles page shows breadcrumb with "Articles"`: `page.goto('/articles')`. | Delete (page removed, R-27). |
| `e2e/responsive.spec.ts:42-43` | `articles page renders without horizontal overflow`: `page.goto('/articles')`. | Delete. |
| `e2e/responsive.spec.ts:82-85` | Touch-target check: `card-articles` bounding box height at least 44. | Does not visit `/articles`. Keep if the link survives as that element, retargeted to the shipped selector (Gap G2). |
| `e2e/responsive.spec.ts:106` | `card-articles` is visible. | Same (Gap G2). |
| `e2e/accessibility.spec.ts:77-78` | `toHaveAttribute('href', '/articles')` on `card-articles`. Asserts a link to `/articles`. | Change to the exact `https://mnemra.dev/blog/` href plus `target` and `rel`, or fold into BM-PW-1. |

Target state (R-26 acceptance): `git grep -n '/articles' -- e2e frontend/e2e` returns only lines in tests that assert absence or the redirect. Today it returns `e2e/public-pages.spec.ts:71,166`, `e2e/responsive.spec.ts:43` and `e2e/accessibility.spec.ts:78`. The `card-articles` lines do not contain the string `/articles`, so that grep misses them; T5 also runs the case-insensitive `article` grep above and judges each hit.

## Unit-test hand-off (the implementer's, not edge tests)

For T2 and T3's unit tests; none is repeated through the process boundary:

- R-25: forged `Host`; each of `X-Forwarded-Host`, `-Proto`, `-For`, `-Prefix`; `/articles/`, `/articles%2Fx`, `/articles?next=https://example.com/`; HEAD, OPTIONS, PATCH; `/articlesx` and `//articles` not 301; `/articles/%zz`; nothing from path, query, `Host` or `X-Forwarded-*` reaching the response.
- R-26: a component test per site asserting `href`, `target`, `rel`; the spec's source-tree grep for `/articles`; intake SC-9's search.
- R-27: the removed routes' remaining methods (POST, PUT, DELETE, with and without a session) and `/api/articles/{slug}`; the existing admin login tests passing unchanged; `/admin/articles` handled as any unmatched admin path; the admin sidebar and dashboard component tests.
- R-28: `should_track` false for `/articles` and `/articles/x`, true for `/projects`.

## Spec gaps

Each: the requirement, why it cannot yet produce a testable scenario, what would fix it. None blocks this plan; G2 and G3 bound how strict T5 can be.

- **G1. Binary start-up contract unstated.** R-25 to R-28 and the plan name no start command, port variable or environment list for the built binary. T5's plan line names `STATIC_DIR`, `DATABASE_URL`, a list file, `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`; `CLAUDE.md` names port 3000. Fix: T5 takes the start command and port setting from the built artifact's `--help` and error text and says so in the test file's header, or T2 records them.
- **G2. The Articles link's selector after the move is unspecified.** R-26 fixes `href`, `target` and `rel` at four sites, not whether the link stays a `card-articles` element. Existing specs select by that testid and assert a 44px touch target on it. Fix: the spec or T3 states the surviving selector; absent that, T5 selects by role and accessible name.
- **G3. The origin lock's refusal is unspecified except in First run.** R-23 says "the origin lock's refusal"; `## First real run` says "the origin lock's 404". Status, body and headers are not given as a contract. Fix: pin status and body in the spec. Until then BM-PY-3 asserts equality with the refusal an unrelated path gets without the header, plus "not a 301".
- **G4. The admin dashboard API path is unnamed.** R-27 names the response, not its URL. Fix: name the URL in the spec; or T5 reads it from the admin UI's network calls in a browser, declared in the test header.
- **G5. R-28 has no externally observable surface.** It is asserted as a function result only. Fix, if an edge check is wanted: name the page-hit read-back surface.
- **G6. The spec's Constraints names no edge-test form.** The form here is plan D4's (P-TDDPairs order). Fix: add a Constraints line.
- **G7. The `justfile` comment on `e2e` says the app must be on localhost:8080.** `CLAUDE.md` `<gates>` and `playwright.config.ts` say Vite on :5173 proxying to :3000. Not a spec matter; this plan follows `CLAUDE.md`. Fix: correct the comment.

## Not covered by edge tests

- R-25's remaining rows, R-26's per-site component tests, R-27's other methods, R-28: unit tests (hand-off above).
- A request for `/articles` straight to the raw Fly origin without the edge token (`## First real run`, R-23): production only, T6's first run. BM-PY-3 is its local counterpart.
- Merge 2 requirements (R-1 to R-24 other than the `/articles` clause, R-29 to R-36): the live-lists plan.
