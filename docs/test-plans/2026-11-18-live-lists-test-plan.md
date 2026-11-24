# Test plan: the live lists (merge 2 of #4853)

- **Delivery:** `live-lists` (plan task T7; this document is T7). Consumer: T17 (edge tests), which writes one test per area id below, once each.
- **Derived from:** `docs/specs/2026-11-18-live-lists-and-blog-move.md` (locked), the plan's T7, T17, D4 and D7, and folio's `CLAUDE.md` `<gates>`. Nothing else. No implementation source was read and none is cited. T8's code did not exist when this was written. The only test code consulted is the header of `tests/edge/harness.py` (what the blog-move suite already declared it read) and the head of `e2e/projects.spec.ts` (what it mocks today).

## Requirements in scope

R-5 (the spawned-binary bullets), R-16 (restart serves held copies; start answers health), R-18 and R-19 (the switch), R-21, R-22, R-23's two `/api/lists/` routes, R-24 (page), R-29's bullets that reach a binary or a page (no list file; `check-projects` retired; `/projects.json` retired), and the API Contract. Merge 1's R-25 to R-28 stay in `docs/test-plans/2026-11-18-blog-move-test-plan.md` and `tests/edge/test_blog_move.py`; T17 does not repeat them (the whole suite still runs, so `/articles` 301 and `/api/articles` 404 stay checked).

## The binary has no source override (R-5's note), and what that means for every assertion

The shipped binary fetches only the three fixed URLs, and nothing at the process boundary can redirect, replace or fake them: no argument, no environment variable, and the client ignores the proxy variables (R-2). So during every binary-driven test the refresher may reach the real GitHub and mnemra.dev, or fail (offline, rate-limited, sandbox), and the test cannot choose which. **Every Python assertion below holds in both worlds.** The rules T17 follows:

- Assert on a list's content only where the test itself supplied it (a seeded `live_lists` row) or where the switch makes it deterministic (a disabled list is never fetched, so reads as no data yet).
- Where a list may be held or missing depending on the real sources, the allowed set is stated, e.g. "200 with a valid document, or 503 with the exact body". An assertion on a specific repo, post, count or time value from a live fetch is out of bounds.
- Anything that needs a fake source (fetch failures keeping a held copy, a changed description appearing after 6 h, parser fixtures, header sets, redirects refused, sentinels in logs, the refresher's panic) is **not binary-observable**. It is listed under "Not covered by edge tests" and is the implementer's in-process test (R-5: start function, test transport, both clocks), not an edge test.
- **Outbound budget.** Each start with a list enabled spends about 2 unauthenticated GitHub requests against 60 per hour per egress IP (spec Decision levels; carried DD-review F-12). T17 counts the starts that leave both lists enabled and records the number in its report. Where the property under test is routing and not the lists (LL-PY-5 to LL-PY-8, LL-PY-15, LL-PY-16), T17 MAY set `LIVE_LISTS_DISABLED=projects,articles` to avoid the outbound requests, and says so in the test's comment. Areas that test the default state, the lists routes' shape and the no-list-file start leave the variable unset.

## Forms

| Surface | Form | Run |
|---|---|---|
| The `folio` binary over HTTP and as a process (routes, switch, start, retired subcommand, retired path) | Python stdlib `unittest` + `subprocess` + `http.client` + `sqlite3` (stdlib, to seed and read back the store), in `tests/edge/`, extending `tests/edge/harness.py` (plan D4) | "Run: Python suite" |
| The `/projects` page in a browser | Playwright TypeScript, root `e2e/projects.spec.ts`, `retries: 0`, API mocked | "Run: `just e2e`" |

No third-party Python package, no Gherkin runner. Given/When/Then goes in test names and body comments. The spec's Constraints names no edge form (Gap G8); the form is plan D4's (P-TDDPairs order).

## Areas

Each area is one way in, tested once. The ids are for T17's test names. Union check: every id below appears in exactly one runner's test names; T17 runs `grep -rhoE 'LL-(PY|PW)-[0-9]+' tests/edge e2e | sort -u` and compares it to this table's ids (a row dropped from both runners would otherwise read as green).

### Python suite (built binary, over HTTP and as a process)

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| LL-PY-1 | Start with no list file | R-29 (last bullet), R-5, R-16 | Given a `STATIC_DIR` holding `index.html` and no `projects.json`, a fresh database, the variable unset. When the binary is spawned. Then `GET /api/health` answers 200 within 15 s of spawn (poll against the timeout, no fixed sleep), and the process is still running after that answer. |
| LL-PY-2 | `GET /api/lists/projects`, shape | R-21, API Contract | Given a started server, the valid `X-Folio-Edge-Auth`, the variable unset. When the route is read. Then `Cache-Control` is exactly `no-store` and `Content-Type` exactly `application/json`, and either (a) 200 with a body whose top-level keys are exactly `fetched_at` and `groups`, `fetched_at` matching `YYYY-MM-DDTHH:MM:SS.sssZ`, each group an object with a non-empty string `owner` and a `repos` array of objects with exactly string `name` and string `description`; or (b) 503 with a body equal, as parsed JSON, to `{"error":"Projects list unavailable"}`. Nothing else is allowed. No assertion on which repos or how many. |
| LL-PY-3 | `GET /api/lists/freshness`, shape | R-22, API Contract | Given a started server, the valid token. When the route is read. Then 200, `Cache-Control` exactly `no-store`, body parses as a JSON object whose key set equals `{projects, articles, started_at}`, `started_at` a string in the `fetched_at` grammar, `projects` and `articles` each `null` or a string in that grammar. Never 503 (the route has no unavailable state). `Content-Type` is not pinned (the contract names only `no-store`). |
| LL-PY-4 | `started_at` fixed before the first request | R-22 | Given T17 records instant A (UTC) immediately before spawn. When health first answers 200 (the first request served), then instant B is recorded, then freshness is read twice. Then `started_at` lies between A and B inclusive, and the two reads return the same `started_at`. (A value set lazily on the freshness request is after B; a value that changes per request differs between reads.) The check ranges over a real wall clock, so A is taken before spawn and the comparison is at millisecond resolution with the grammar's precision; no tolerance is added. |
| LL-PY-5 | Origin lock on `/api/lists/projects` | R-23 (merge 2) | Given a server with an edge token set. When the route is requested without `X-Folio-Edge-Auth`. Then the response equals, in status and body, what an unrelated path (a random nonexistent one) returns without the header (Gap G3), and it is not the route's own answer. Control in the same test: the same route with the valid header answers 200 or 503 (not 404), so the refusal is the lock and not a missing route. |
| LL-PY-6 | Origin lock on `/api/lists/freshness` | R-23 (merge 2) | Same as LL-PY-5 for freshness; control is 200 with the valid header. |
| LL-PY-7 | Rate limiter refuses `/api/lists/projects` once exhausted | R-23 (merge 2) | Given a fresh server and the valid token. When requests to `/api/lists/freshness` are sent until one is refused (bounded count, Gap G2), then `/api/lists/projects` is requested. Then it gets that same refusal (equal status and body to the burst's refusal), not 200 or 503. Conditional on Gap G2: if no refusal appears within the bound, T17 reports the area unreachable and does not invent a limit. |
| LL-PY-8 | Rate limiter refuses `/api/lists/freshness` once exhausted | R-23 (merge 2) | Mirror of LL-PY-7: exhaust through `/api/lists/projects`, then freshness gets the refusal. Separate test, separate fresh server (no state shared with LL-PY-7). |
| LL-PY-9 | Switch `projects`: the list is cleared and not served | R-18, R-19, Scenarios (switch) | Given a database seeded with a valid projects row and a valid articles row (see "Fixtures"), `LIVE_LISTS_DISABLED=projects`. When the server starts and is read. Then `/api/lists/projects` is 503 with the exact body above and `no-store`; freshness `projects` is `null`; after the server is stopped the `live_lists` table has no `projects` row; and the log holds a line naming `projects` and the outcome word `deleted` (wording unspecified, Gap G4). The articles half ("behave normally") is source-dependent and not asserted. |
| LL-PY-10 | Switch `articles`: only articles are cleared | R-18, R-19, Scenarios (switch) | Given the same seeded database, `LIVE_LISTS_DISABLED=articles`. When the server starts and is read. Then freshness `articles` is `null`; `/api/lists/projects` is 200 with a valid document (the seeded projects row survives the delete, and any refresh keeps a valid document); freshness `projects` is non-null; after stop the `live_lists` table has no `articles` row and still has a `projects` row; the log holds a line naming `articles` and `deleted`. |
| LL-PY-11 | Switch absent: toggle off at the default, held copies served from the first request | R-18 (absent), R-16 | Given the seeded database and the variable unset. When the server starts and the first request after health is `/api/lists/projects`. Then 200 with a valid document, freshness `projects` and `articles` both non-null, and after stop both rows still exist in `live_lists`. This is the "off" half of the toggle pair whose "on" halves are LL-PY-9, LL-PY-10 and LL-PY-12. |
| LL-PY-12 | Switch with an unknown token: both lists disabled, `error` logged, server still starts | R-18 (unknown token), R-19 | Given the seeded database and `LIVE_LISTS_DISABLED=articles,projcts`. When the server starts. Then `/api/lists/projects` is 503 with the exact body, freshness `projects` and `articles` are both `null`, health is 200 (the server does not refuse to start), after stop `live_lists` has no rows, and the log holds at least one error-level line that a run with the variable unset, same seeded database and otherwise identical, does not (baseline diff, so unrelated lines cannot satisfy it; Gap G4). |
| LL-PY-13 | Switch with a value that is not valid Unicode: read as set, never as absent | R-18 (non-Unicode), R-5 (raw OS value) | Given the seeded database and `LIVE_LISTS_DISABLED` set, in the child's environment, to the bytes `b"articles\xff"` (POSIX; set through `os.environb`). When the server starts. Then the same observations as LL-PY-12. This is the only place the raw-OS-value clause is visible from outside, because a lossy read in `main.rs` would show here as "nothing disabled" (the seeded rows survive, both routes answer 200). |
| LL-PY-14 | `folio check-projects` is gone | R-29 (subcommand) | Given a `STATIC_DIR` holding a valid, fresh `projects.json` (the harness's existing list file, `fetched_at` now), the shape on which the retired subcommand would exit 0. When `folio check-projects` runs (bounded by a timeout). Then it exits non-zero and does not start serving. Exit code and message wording are unspecified (Gap G6); only non-zero is asserted, and the fresh list file is what makes it mean "removed" and not "stale list". |
| LL-PY-15 | `/projects.json` carries no special cache rule, file present | R-29 (static `/projects.json` and its no-cache rule) | Given a `STATIC_DIR` holding `projects.json` and a control file of the same kind (`control.json`), valid token. When both are read. Then `Cache-Control` on `/projects.json` contains no `no-cache` and equals the control's `Cache-Control` (both absent counts as equal). If the control also carries `no-cache`, T17 reports it (Gap G7) rather than weakening the first half. |
| LL-PY-16 | `/projects.json` carries no special cache rule, file absent | R-29 (as above) | Given a `STATIC_DIR` with no `projects.json`, valid token. When `/projects.json` and a nonexistent control `/control-<random>.json` are read. Then neither response carries `no-cache` and their `Cache-Control` values are equal. Catches a path-scoped rule that fires without a file behind it, which LL-PY-15 cannot. Status and body are not pinned (Gap G7). |

### Playwright (`just e2e`, browser, API mocked)

The shared layout mock (`mockPublicApi` in `e2e/fixtures.ts`) stays; the spec mocks `**/api/lists/projects` only, never `projects.json`. The oracle for the loaded state is the committed `frontend/scripts/projects-fixtures/list-expected.json`, read as data, as the existing spec does.

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| LL-PW-1 | Loaded list | R-24 (valid 200), adopted rendering | Given the route mocked to 200 with the fixture document. When `/projects` loads. Then one link per repo with its exact `github.com` href (the existing spec's assertion), no `No projects yet.`, no unavailable line; and the page requested `/api/lists/projects` and never requested `projects.json` (observe via `page.on('request')`). |
| LL-PW-2 | Pending | R-24 (pending) | Given the route held (not fulfilled). When `/projects` loads. Then the heading shows; the text `No projects yet.` and the unavailable line are both absent. The route is then released so teardown is clean. The absence assertions use an expect that holds, not a fixed sleep. |
| LL-PW-3 | 503 shows the unavailable line | R-24 (every other outcome, via the API's own 503) | Given the route mocked to 503 with `{"error":"Projects list unavailable"}`. When `/projects` loads. Then no repo links; exactly the text `The project list is temporarily unavailable; try again shortly.` in an element with `role="alert"`; followed by two links with `href` exactly `https://github.com/mnemra` and `https://github.com/prmanahan`, distinct link text each naming its owner, each `target="_blank"` and `rel="noopener noreferrer"`. |
| LL-PW-4 | Unsettled after 15 s: aborted, late response never applied | R-24 (15 s) | Given the route held and the page clock controlled (`page.clock`, so no real 15 s wait and no hardcoded sleep). When the clock advances past 15 s, then the held request is fulfilled with a valid 200 document. Then the unavailable line is showing before and after the fulfil, no repo links ever render, and `No projects yet.` never shows. If the Playwright version in the repo lacks the clock API, T17 reports it and proposes the smallest alternative; it does not fall back to a real 15 s wait without saying so. |
| LL-PW-5 | A hostile description renders as text | R-24 (`{@html}` acceptance), DSP-57 | Given the route mocked to 200 with one repo whose description is `<img src=x onerror=alert(1)> **b**`. When `/projects` loads. Then the page holds no `img` or `strong` element inside the list region, and the description text is present verbatim as text. |

R-24's other outcomes (non-JSON body, wrong shape, network error, other statuses) and its ordering inside the component are component tests (T12), not edge areas: each reaches the same unavailable line the 503 area already reaches through the page.

## Fixtures (built per test, by code)

- **Seeded rows.** The harness starts the binary once with `LIVE_LISTS_DISABLED=projects,articles` (so no outbound request and no rows written), waits for health, stops it, and inserts rows into `live_lists(name, document)` with stdlib `sqlite3`. The table exists because migration `008` ran at that start. The table name, columns and document shapes come from the spec's Data Model and R-11, nothing else. Then it starts the binary under test against that database file.
  - Projects document: the committed `list-expected.json` fixture (data only) with `fetched_at` set to the current instant in the grammar.
  - Articles document: one post built by hand from the Data Model (`posts[].title`, `summary`, `link` under `https://mnemra.dev/blog/<slug>/`, `published`) with `fetched_at` now. If the server rejects it at load, freshness shows `null` and LL-PY-11 fails loudly, which is the signal that the fixture, not the product, is wrong; T17 then checks the document against R-11 before reporting.
- **Per-test state.** Own temp directory, `STATIC_DIR`, database, free port, generated `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`; none committed.
- **Log capture.** The harness's existing log capture of the process. Level and line format are read from one real run and declared in the test file's header (Gap G4).

## Run: Python suite

```sh
uv python install --no-config --no-bin 3.12
PY="$(uv python find --no-config --system --managed-python '>=3.12')"
FOLIO_BIN=/absolute/path/to/built/folio \
  uv run --no-config --no-project --python "$PY" -m unittest discover -v -s tests/edge -t tests/edge
```

`FOLIO_BIN` is the blog-move suite's variable. The harness fails, not skips, when it is unset, missing, not executable or does not answer `--help`; a run exiting 5 (no tests ran) is a failure. The command, the binary's commit and the result go in merge 2's report (D4). The whole `tests/edge` suite runs, so merge 1's areas run again against merge 2's binary.

## Run: `just e2e`

```sh
just e2e                                  # root playwright.config.ts over e2e/, chromium + firefox + webkit
npx playwright test e2e/projects.spec.ts  # this plan's one spec
```

Per `CLAUDE.md` `<gates>`: Vite on :5173 is started by Playwright's `webServer` and proxies `/api` to the backend on :3000; LL-PW-1 to LL-PW-5 mock the API and need no backend. Playwright runs in no CI job, so the run is recorded in merge 2's report. Because that run "cannot fail mechanically" (carried DD-review G-4), the record names the file, the three browsers, and the per-project pass count; a record that says only "passed" is not accepted.

## Unit-test hand-off (the implementer's, not edge tests)

Edge cases noticed while planning, for T8 to T12:

- R-18's remaining spellings: `projects,articles`, `Articles`, `projcts`, `" articles ,"`, empty, whitespace-only, empty segment; through the start function, as the spec says.
- R-19's `failed` delete outcome (forced failure), the log level of that line, and re-enabling on unset plus restart.
- R-21's 200 body equal to the held document; R-22's injected-clock check; R-16's skew boundary (exactly the allowance loads, 1 ms beyond does not) and a row edited to fail validation.
- Everything on the sources, parser, client, logs, refresher and held copy (R-1 to R-17, R-20, R-30), which needs a fake source.
- R-24's remaining outcomes and the `{@html}` grep.

## Spec gaps

Each: the requirement, why it cannot yet produce a testable scenario, what would fix it. G1 and G8 repeat the blog-move plan's G1 and G6. G2 and G3 bound LL-PY-5 to LL-PY-8; G4 bounds the log assertions. None blocks the plan.

- **G1. Binary start-up contract unstated.** R-5 names `GET /api/health` and a start function but no start command, port variable or environment list for the spawned binary. The blog-move harness already declares what it read for these. Fix: the spec or T8 records them; until then T17 reuses the harness's declared reads and adds `LIVE_LISTS_DISABLED` (named by R-18) and the database file, which it must read from the same declared source.
- **G2. The global rate limiter's parameters and refusal are unspecified.** R-23 says "with the global rate limiter exhausted, each route gets the limiter's refusal", but no limit, window, keying (global versus per client), refusal status or body, whether `/api/health` counts, or order against the origin lock is given. A test can only discover them by bursting, which is an unbounded search with a guessed bound and a possibly-fast refill. Fix: pin the limiter's numbers and refusal in the spec, or in a test-facing line of the plan. Until then LL-PY-7 and LL-PY-8 compare the refusal to the burst's own refusal and report "unreachable" if none appears in a stated bound; they do not assert a status code.
- **G3. The origin lock's refusal is unspecified except in First run.** R-23 says "the origin lock's refusal"; First run says "the origin lock's 404". Same as blog-move G3. Fix: pin status and body. Until then LL-PY-5 and LL-PY-6 compare against an unrelated path's refusal and require the with-header control to differ.
- **G4. Log observation surface is unspecified.** R-18 and R-19 require an `error` line and a start line naming the disabled set and `deleted`, `absent` or `failed`, but give no format, stream, field names or whether the process's stdout or stderr carries them. R-30's refresh summary line (`start` trigger, per-list outcome and count) is source-dependent in content and unspecified in format, so it has no binary-observable case here. Fix: state the logging format and stream, and the exact start-line fields. Until then LL-PY-9, LL-PY-10 and LL-PY-12 assert on a line holding the list name and the spec's own outcome word, and on a baseline-diffed error-level line, with the format declared in the test header after one observed run.
- **G5. Binary-observable limit of R-5's spawned-binary test.** The binary has no source override, so a start function that awaited the first refresh would pass LL-PY-1 whenever the real sources answer inside 15 s (carried DD-review G-6). Fix: the in-process test with a never-answering source (R-5, first acceptance bullet) is the holder; this plan does not claim it.
- **G6. `check-projects` after retirement.** R-29 says the subcommand is retired but gives no exit code or message for invoking it. Fix: say whether it is "an unknown subcommand" and name the exit code. Until then LL-PY-14 asserts non-zero only.
- **G7. "`/projects.json` no longer special" has no positive definition.** R-29 retires the file and its no-cache rule, but what a request for the path returns afterwards (static file, SPA fallback, 404) is unstated. LL-PY-15 and LL-PY-16 therefore assert absence of `no-cache` plus equality with a control path, and cannot detect other leftovers. Fix: state the expected response for the present and absent cases.
- **G8. The spec's Constraints names no edge-test form.** The form here is plan D4's. Fix: add a Constraints line.
- **G9. R-24's 15 s abort is observable in a browser only through a controlled clock.** The spec does not say the browser test may fake time. LL-PW-4 uses Playwright's clock; if unavailable, T17 reports. Fix: allow it in R-24's acceptance.

## Not covered by edge tests

- **R-1 to R-4, R-6 to R-15, R-17, R-30 (content)**: the client, parser, validator, store and refresher need a fake or hostile source, which the binary cannot be given. Held by the in-process tests of T8 and neighbours. Not claimed here.
- **R-16's "sources failing" and "sources hanging" variants and R-17's six-hourly tick**: need to control the sources and the clock. LL-PY-11 covers the part that does not (seeded rows served from the first request) and LL-PY-1 the 15 s health answer.
- **R-19's "unset and restart refills"** and the "articles behave normally" and "projects behave normally" clauses: the refill is source-dependent; no binary-observable property distinguishes it from a failed fetch.
- **R-20 (both prompts)**: the prompts are reached only through the AI endpoints, which need a model credential and an outbound call to Anthropic; the binary has no override for that either. Unit tests (T8 and the AI path).
- **R-31 to R-35 (the Node checks and workflow assertions) and R-36 (audit)**: not binary or browser surfaces; vitest, the workflow test and the `audit.yml` run hold them (D7).
- **First-real-run items** (runbook drill, scheduled red alert path, Fly egress, log retention): production only, T18.
- **A merge-2 `fly.toml` with no `release_command`, the old spec's superseded line, the `git grep` retirements**: repository greps, T-tasks' acceptance, not edge tests.
