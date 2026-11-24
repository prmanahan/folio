# Test plan: the live lists (merge 2 of #4853)

- **Delivery:** `live-lists` (plan task T7; this document is T7). Consumer: T17 (edge tests), which writes one test per area id below, once each.
- **Derived from:** `docs/specs/2026-11-18-live-lists-and-blog-move.md` (locked), the plan's T7, T17, D4 and D7, and folio's `CLAUDE.md` `<gates>`. The first draft (`f5eb176`) was written from those alone and read no source. **Revision 1** (this one) closes Cerberus's review, whose findings quote the *shipped, pre-merge-2* code at `c48ffbb` for the existing rate limiter, origin lock, log lines and static serving. Those facts are **inherited from that review, not re-verified by the plan author**; each carries its source path so T17 can confirm it on the built binary before relying on it, and declares the read in the test header. Nothing here reads T8's code (none exists).
- **Consumer rule for inherited facts:** where a fact below says "(review, `path`)", the number or string is a pin T17 asserts. If the built binary disagrees, that is a failure to report (code or plan wrong, dispatcher decides), never a silent adjustment.

## Requirements in scope

R-5 (the spawned-binary bullets), R-16, R-18 and R-19 (the switch), R-21, R-22, R-23's two `/api/lists/` routes, R-24 (page), R-29's bullets that reach a binary or a page (no list file or an invalid one; `check-projects` retired; `/projects.json` retired), and the API Contract. Merge 1's R-25 to R-28 stay in `docs/test-plans/2026-11-18-blog-move-test-plan.md` and `tests/edge/test_blog_move.py`; the whole suite still runs, so `/articles` 301 and `/api/articles` 404 stay checked.

## The binary has no source override (R-5's note), and what that means for every assertion

The shipped binary fetches only the three fixed URLs, and nothing at the process boundary can redirect, replace or fake them: no argument, no environment variable, and the client ignores the proxy variables (R-2). So during every binary-driven test the refresher may reach the real GitHub and mnemra.dev, or fail, and the test cannot choose which. **Every Python assertion below holds in both worlds.** The rules T17 follows:

- Assert on a list's content only where the test supplied it (a seeded `live_lists` row) or where the switch makes it deterministic (a disabled list is never fetched, so reads as no data yet).
- Where a list may be held or missing depending on the real sources, the allowed set is stated. An assertion on a specific repo, post, count or time from a live fetch is out of bounds.
- Anything needing a fake source (fetch failures keeping a held copy, a changed description after 6 h, parser fixtures, header sets, redirects refused, sentinels in logs, the refresher's panic) is **not binary-observable**; see "Not covered by edge tests".
- **Outbound budget.** Each start with a list enabled spends about 2 unauthenticated GitHub requests against 60 per hour per egress IP (carried DD-review F-12). T17 counts the starts that leave both lists enabled and records the number in its report. For the routing-only areas LL-PY-5 to LL-PY-8, LL-PY-14 to LL-PY-18 the variable `LIVE_LISTS_DISABLED=projects,articles` is **mandatory** (finding 2: with the lists enabled a real fetch could change what the route answers; disabled, the lists routes answer deterministically). The default-state areas (LL-PY-1 to LL-PY-4, LL-PY-11) leave it unset.

## Forms

| Surface | Form | Run |
|---|---|---|
| The `folio` binary over HTTP and as a process | Python stdlib `unittest` + `subprocess` + `http.client` + `sqlite3` (stdlib, to seed and read back the store), in `tests/edge/`, extending `tests/edge/harness.py` (plan D4) | "Run: Python suite" |
| The `/projects` page in a browser | Playwright TypeScript, root `e2e/projects.spec.ts`, `retries: 0`, API mocked | "Run: `just e2e`" |

No third-party Python package, no Gherkin runner. Given/When/Then goes in test names and body comments. The spec's Constraints names no edge form (Gap G8); the form is plan D4's (P-TDDPairs order).

## Areas

Each area is one way in, tested once. Test names carry the id, written `test_LL_PY_<n>_<behaviour>` for Python and `test('LL-PW-<n> <behaviour>', ...)` for Playwright.

**Union check (finding 6).** T17 runs both, from the repository root, and records the output:

```sh
{ grep -rhoE 'def test_LL_PY_[0-9]+' tests/edge | sed -E 's/def test_//; s/_/-/g'
  grep -rhoE "test\('LL-PW-[0-9]+" e2e | sed -E "s/test\('//"; } | sort | uniq -c
```

Every plan id (LL-PY-1 to LL-PY-18, LL-PW-1 to LL-PW-5) must appear with a count of exactly 1: a missing id and a duplicated id both fail. (The `sed` maps the Python `_` spelling onto the plan's `-` spelling, so a test named with either separator in the id is matched once; a name that spells the id some other way matches nothing and shows as missing.)

### Python suite (built binary, over HTTP and as a process)

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| LL-PY-1 | Start with no list file | R-29 (last bullet), R-5, R-16 | Given a `STATIC_DIR` holding `index.html` and no `projects.json`, a fresh database, the variable unset. When the binary is spawned. Then `GET /api/health` answers 200 within 15 s of spawn (poll against the timeout, no fixed sleep), and the process is still running after that answer. |
| LL-PY-2 | `GET /api/lists/projects`, shape | R-21, API Contract | Given a started server, the valid `X-Folio-Edge-Auth`, the variable unset. When the route is read. Then `Cache-Control` is exactly `no-store` and `Content-Type` exactly `application/json`, and either (a) 200 with a body whose top-level keys are exactly `fetched_at` and `groups`, `fetched_at` matching `YYYY-MM-DDTHH:MM:SS.sssZ`, each group an object with a non-empty string `owner` and a `repos` array of objects with exactly string `name` and string `description`; or (b) 503 with a body equal, as parsed JSON, to `{"error":"Projects list unavailable"}`. Nothing else is allowed. No assertion on which repos or how many. Call this body+header contract "the LL-PY-2 (a) contract" and "the LL-PY-2 (b) contract". |
| LL-PY-3 | `GET /api/lists/freshness`, shape | R-22, API Contract | Given a started server, the valid token. When the route is read. Then 200, `Cache-Control` exactly `no-store`, body parses as a JSON object whose key set equals `{projects, articles, started_at}`, `started_at` a string in the `fetched_at` grammar, `projects` and `articles` each `null` or a string in that grammar. Never 503. `Content-Type` is not pinned (the contract names only `no-store`). |
| LL-PY-4 | `started_at` fixed before the first request | R-22 | Given T17 records instant A (UTC) immediately before spawn. When health first answers 200 (the first request served), then instant B is recorded, then freshness is read twice. Then `started_at` lies between A and B inclusive, and the two reads return the same `started_at`. No tolerance is added. |
| LL-PY-5 | Origin lock on `/api/lists/projects` | R-23 (merge 2), First run | Given a server with an edge token set and `LIVE_LISTS_DISABLED=projects,articles`. When the route is requested without `X-Folio-Edge-Auth`. Then the response is **404 with an empty body** (spec First run; review, `origin_lock.rs`), and its `(status, body, content-type)` triple equals that of an unrelated nonexistent path requested without the header. Control in the same test: the same route with the valid header answers 503 (the lists are disabled, so exactly the LL-PY-2 (b) contract), so the 404 is the lock and not a missing route. |
| LL-PY-6 | Origin lock on `/api/lists/freshness` | R-23 (merge 2), First run | Same as LL-PY-5 for freshness. Control: 200 with the valid header and the LL-PY-3 shape. |
| LL-PY-7 | Rate limiter refuses `/api/lists/projects` once the limit is spent | R-23 (merge 2) | See "Rate-limit pin" below. Given a fresh server (`LIVE_LISTS_DISABLED=projects,articles`, valid edge token, no admin Bearer). When the limiter is spent by requests through `/api/profile`, then `/api/lists/projects` is requested. Then it answers exactly the pinned 429, `Retry-After: 60` and body. |
| LL-PY-8 | Rate limiter refuses `/api/lists/freshness` once the limit is spent | R-23 (merge 2) | Same as LL-PY-7 for freshness, a separate test on its own fresh server (no state shared). The refusal is the pinned 429 and never a 503 or a 200 (finding 2). |
| LL-PY-9 | Switch `projects`: the list is cleared and not served | R-18, R-19, Scenarios (switch) | Given a database seeded with a valid projects row and a valid articles row (see "Fixtures"), `LIVE_LISTS_DISABLED=projects`. When the server starts and is read. Then `/api/lists/projects` is the LL-PY-2 (b) contract (503, exact body, `no-store`); freshness `projects` is `null` and **freshness `articles` is non-null**; after the server is stopped the `live_lists` table has no `projects` row and **still has the `articles` row**; and the log holds a line naming `projects` and the outcome word `deleted` (wording unspecified, Gap G4). Only the seeded articles row can make `articles` non-null deterministically, which is why the articles half is asserted as survival of the seed, not as a fetch. |
| LL-PY-10 | Switch `articles`: only articles are cleared | R-18, R-19, Scenarios (switch) | Given the same seeded database, `LIVE_LISTS_DISABLED=articles`. When the server starts and is read. Then freshness `articles` is `null`; `/api/lists/projects` satisfies **the full LL-PY-2 (a) contract, headers included** (200, `no-store`, `application/json`, the document shape), and **if its `fetched_at` equals the seeded one the body equals the seeded document exactly** (a refresh may have replaced it, in which case only the shape is asserted); freshness `projects` is non-null; after stop the `live_lists` table has no `articles` row and still has a `projects` row; the log holds a line naming `articles` and `deleted`. |
| LL-PY-11 | Switch absent: toggle off at the default, held copies served from the first request | R-18 (absent), R-16 | Given the seeded database and the variable unset. When the server starts and the first request after health is `/api/lists/projects`. Then the full LL-PY-2 (a) contract (headers included), with the same "equals the seeded document if `fetched_at` equals the seeded one" rule as LL-PY-10; freshness `projects` and `articles` both non-null; after stop both rows still exist in `live_lists`. This is the "off" half of the toggle pair whose "on" halves are LL-PY-9, LL-PY-10, LL-PY-12 and LL-PY-13. |
| LL-PY-12 | Switch with an unknown token: both lists disabled, `error` logged, server still starts | R-18 (unknown token), R-19 | Given the seeded database and `LIVE_LISTS_DISABLED=articles,projcts`. When the server starts. Then `/api/lists/projects` is the LL-PY-2 (b) contract, freshness `projects` and `articles` are both `null`, health is 200, after stop `live_lists` has no rows, and the log holds an error-level line that **names the switch or the bad token** (`LIVE_LISTS_DISABLED` or `projcts`; Gap G4). Baseline comparison is **normalised** (finding 3): both runs' logs have the timestamp and ANSI colour codes stripped, each line reduced to `(level, message, fields)`, and the set of error-level triples in the switched run must contain at least one triple absent from the unset-variable baseline's set. The harness start already emits error lines unrelated to the switch (review: `check_csp_meta`, and `load_csp_header` when `csp-header.txt` is missing), so a raw line diff would pass vacuously. The "names the switch or the token" assertion is the primary one; the normalised diff is the guard that it is the switch's line and not an unrelated one. T17 states the normalisation in the test header. |
| LL-PY-13 | Switch with a value that is not valid Unicode: read as set, never as absent | R-18 (non-Unicode), R-5 (raw OS value) | Given the seeded database and the child's environment mapping, the one the harness passes to `Popen` (review, `harness.py:88-96`), holding `b"LIVE_LISTS_DISABLED": b"articles\xff"` (bytes keys and values, POSIX). The mapping is built that way, **not** through `os.environb`. When the server starts. Then the same observations and the same normalised error-line rule as LL-PY-12. This is the only place the raw-OS-value clause is visible from outside: a lossy read in `main.rs` would show as "nothing disabled" (the seeded rows survive, both routes answer 200). |
| LL-PY-14 | `folio check-projects` is gone | R-29 (subcommand) | Given a `STATIC_DIR` holding a valid, fresh `projects.json` (the harness's existing list file, `fetched_at` now), the shape on which the retired subcommand would exit 0. When `folio check-projects` runs (bounded by a timeout). Then it exits non-zero and does not start serving. Only non-zero is asserted (Gap G6); the fresh list file is what makes it mean "removed" and not "stale list". |
| LL-PY-15 | `/projects.json` carries no special cache rule, file present | R-29 (static `/projects.json` and its no-cache rule) | Given a `STATIC_DIR` holding `projects.json` and a control file of the same kind (`control.json`) with the same body, valid token, `LIVE_LISTS_DISABLED=projects,articles`. When both are read. Then `/projects.json` has **status, `Content-Type` and `Cache-Control` each equal to the control's**, and its `Cache-Control` contains no `no-cache`. If the control also carries `no-cache`, T17 reports it (Gap G7) and does not weaken the second half. |
| LL-PY-16 | `/projects.json` carries no special cache rule, file absent | R-29 (as above) | Given a `STATIC_DIR` with no `projects.json`, valid token, the switch as above. When `/projects.json` and a nonexistent control `/control-<random>.json` are read. Then **status, `Content-Type`, `Cache-Control` and body are each equal between the two** (both fall to the same fallback; review, `static_files.rs:205`, says that is the SPA fallback), and neither `Cache-Control` contains `no-cache`. Catches a path-scoped rule that fires without a file behind it, which LL-PY-15 cannot. |
| LL-PY-17 | Start with a stale list file | R-29 ("or invalid" clause of the startup refusal) | Given a `STATIC_DIR` holding a `projects.json` whose `fetched_at` is more than 30 minutes old (the old spec's bound; the old refusal fired on it), a fresh database. When the binary is spawned. Then `GET /api/health` answers 200 within 15 s of spawn and the process is still running. The switch is `projects,articles`. |
| LL-PY-18 | Start with a malformed list file | R-29 (as above) | Given a `STATIC_DIR` holding a `projects.json` that is not valid JSON (for example the bytes `{not json`). When the binary is spawned. Then the same as LL-PY-17: health 200 within 15 s, process still running. Switch as above. |

### Rate-limit pin (finding 1; closes Gap G2)

The numbers below are the shipped limiter's (review, `libs/site-core/middleware/global_rate_limit.rs` at `c48ffbb`), inherited and not re-verified by the plan author. T17 **declares the source of these numbers in the test file's header** and confirms them against the built binary through LL-PY-7 and LL-PY-8 themselves; a mismatch is a reported failure.

- Limit: **60 requests per 60 s**, sliding window, keyed on the peer IP. The harness sets no `TRUSTED_IP_HEADER`, so the key is `127.0.0.1`. The state is per process, hence one fresh server per test.
- Refusal: **429**, `Retry-After: 60`, body `{"error":"Too many requests. Please wait before retrying."}`.
- Exempt: a valid admin Bearer. `/api/health` and requests without the edge token never reach the limiter (they are not counted and not refused by it). The burst therefore carries the valid edge token and no Bearer.
- Counted route: `/api/profile` is known to sit behind the limiter (review, `routes/profile.rs:16`), so the burst goes through it.

Steps for LL-PY-7 and LL-PY-8, in order, each a test assertion:

1. Fresh server, `LIVE_LISTS_DISABLED=projects,articles`, valid edge token.
2. **Before any burst**, read the route under test once with the valid token and assert its own answer (LL-PY-7: the LL-PY-2 (b) 503 contract; LL-PY-8: the LL-PY-3 200 shape). This request itself counts toward the 60 (it is behind the limiter), so the burst below is 59 more; T17 counts exactly and states the arithmetic in a comment.
3. Send `/api/profile` requests until the 60th counted request overall; assert every one of requests 1 to 60 is **not** 429 (success or the route's own answer).
4. Request 61 (the next limited request, to `/api/profile` or to the route under test) must be the pinned 429 with `Retry-After` exactly `60` and the exact body. **No refusal by request 61 is a failure**, not a skipped or "unreachable" result.
5. Then request the route under test and assert it answers exactly that 429, `Retry-After: 60` and body (not a 503, not a 200).

The 60-second window means a whole burst must finish within it; if T17's machine cannot send 61 requests in under 60 s, that is a failure to report as inconclusive with the measured rate, not a reason to loosen the numbers.

### Playwright (`just e2e`, browser, API mocked)

The shared layout mock (`mockPublicApi` in `e2e/fixtures.ts`) stays; the spec mocks `**/api/lists/projects` only, never `projects.json`. The oracle for the loaded state is the committed `frontend/scripts/projects-fixtures/list-expected.json`, read as data, as the existing spec does.

| Id | Area (way in) | Requirement | Given / When / Then |
|---|---|---|---|
| LL-PW-1 | Loaded list | R-24 (valid 200), adopted rendering | Given the route mocked to 200 with the fixture document. When `/projects` loads. Then one link per repo with its exact `github.com` href (the existing spec's assertion), no `No projects yet.`, no unavailable line; and the page requested `/api/lists/projects` and never requested `projects.json` (observed via `page.on('request')`). |
| LL-PW-2 | Pending | R-24 (pending) | Given the route held (not fulfilled) and the page clock installed (see LL-PW-4). When `/projects` loads. Then first wait for the request to `/api/lists/projects` to have been made and for the heading to be visible; then advance the clock to just under 15 s (for example 14 s); then assert the text `No projects yet.` and the unavailable line are both absent (so the absence is checked while the request is pending, not before the page rendered). The route is released afterwards for clean teardown. No fixed sleep. |
| LL-PW-3 | 503 shows the unavailable line | R-24 (every other outcome, via the API's own 503) | Given the route mocked to 503 with `{"error":"Projects list unavailable"}`. When `/projects` loads. Then no repo links; exactly the text `The project list is temporarily unavailable; try again shortly.` in an element with `role="alert"`; followed by two links with `href` exactly `https://github.com/mnemra` and `https://github.com/prmanahan`, distinct link text each naming its owner, each `target="_blank"` and `rel="noopener noreferrer"`. |
| LL-PW-4 | Unsettled after 15 s: aborted, late response never applied | R-24 (15 s) | Given the route held and the page clock controlled (`page.clock`, installed before navigation, so no real 15 s wait and no hardcoded sleep). When the clock advances past 15 s, then the held request is fulfilled with a valid 200 document. Then: a `requestfailed` event fired for `/api/lists/projects` (the request was **aborted**), and a fulfil that throws because the request is already gone is the **expected** outcome, caught in the test and not counted as a failure; the unavailable line is showing before and after the fulfil attempt, no repo links ever render, and `No projects yet.` never shows. **Timer precondition (finding 11):** the repo's Playwright (1.58.2, per review) `page.clock` does not fake `AbortSignal.timeout`. So the area requires that the page's 15 s abort is driven by a timer `page.clock` controls (`setTimeout` plus `AbortController`). That is a requirement on T12's implementation, stated here so T12 does not pick `AbortSignal.timeout` and silently break the edge test. If the shipped page uses a timer the clock cannot control, T17 falls back to a **stated real wait of about 16 s** inside the test's own timeout (with the expect polling, not a sleep), says so in the test header, and reports the fallback to the dispatcher. |
| LL-PW-5 | A hostile description renders as text | R-24 (`{@html}` acceptance), DSP-57 | Given the route mocked to 200 with one repo whose description is `<img src=x onerror=alert(1)> **b**`. When `/projects` loads. Then the page holds no `img` or `strong` element inside the list region, and the description text is present verbatim as text. |

R-24's other outcomes (non-JSON body, wrong shape, network error, other statuses) and its ordering inside the component are component tests (T12), not edge areas.

## Fixtures (built per test, by code)

- **Seeded rows.** The harness starts the binary once with `LIVE_LISTS_DISABLED=projects,articles` (no outbound request and no rows written), waits for health, stops it, and inserts rows into `live_lists(name, document)` with stdlib `sqlite3`. The table exists because migration `008` ran at that start. The table name, columns and document shapes come from the spec's Data Model and R-11, nothing else. Then it starts the binary under test against that database file.
  - Projects document: the committed `list-expected.json` fixture (data only) with `fetched_at` set to the current instant in the grammar.
  - Articles document: one post built by hand from the Data Model (`posts[].title`, `summary`, `link` under `https://mnemra.dev/blog/<slug>/`, `published`) with `fetched_at` now. If the server rejects it at load, freshness shows `null` and the tests asserting non-null fail loudly, which signals the fixture, not the product, is wrong; T17 then checks the document against R-11 before reporting.
- **Per-test state.** Own temp directory, `STATIC_DIR`, database, free port, generated `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`; none committed.
- **Log capture and normalisation.** The harness's existing log capture of the process. T17 reads the level and line format from one real run and declares it in the test header. The normalisation used by LL-PY-12 and LL-PY-13: strip the leading timestamp and every ANSI escape sequence, then compare `(level, message, fields)`.

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

Per `CLAUDE.md` `<gates>`: Vite on :5173 is started by Playwright's `webServer` and proxies `/api` to the backend on :3000; LL-PW-1 to LL-PW-5 mock the API and need no backend. Playwright runs in no CI job, so the run is recorded in merge 2's report. Because that run "cannot fail mechanically" (carried DD-review G-4), the record names the file, the three browsers and the per-project pass count; a record that says only "passed" is not accepted.

## Unit-test hand-off (the implementer's, not edge tests)

- R-18's remaining spellings: `projects,articles`, `Articles`, `projcts`, `" articles ,"`, empty, whitespace-only, empty segment; through the start function, as the spec says.
- R-19's `failed` delete outcome (forced failure), the log level of that line, and re-enabling on unset plus restart.
- R-21's 200 body equal to the held document; R-22's injected-clock check; R-16's skew boundary and a row edited to fail validation.
- Everything on the sources, parser, client, logs, refresher and held copy (R-1 to R-17, R-20, R-30), which needs a fake source.
- R-24's remaining outcomes and the `{@html}` grep.
- The timer used for R-24's 15 s abort must be one `page.clock` controls (LL-PW-4's precondition).

## Spec gaps

Each: the requirement, why it cannot yet produce a testable scenario, what would fix it, and (revision 1) what the shipped code answers per the review. G1 and G8 repeat the blog-move plan's G1 and G6.

- **G1. Binary start-up contract unstated.** R-5 names `GET /api/health` and a start function but no start command, port variable or environment list for the spawned binary. The blog-move harness declares what it read. Fix: the spec or T8 records them; until then T17 reuses the harness's declared reads and adds `LIVE_LISTS_DISABLED` (named by R-18) and the database file.
- **G2. The global rate limiter's parameters and refusal are unspecified in the spec. Resolved for the plan by pinning the shipped limiter.** R-23 says "the limiter's refusal" and gives no numbers. The shipped code answers (review, `global_rate_limit.rs` at `c48ffbb`): 60 requests per 60 s, sliding window, keyed on peer IP; 429 with `Retry-After: 60` and body `{"error":"Too many requests. Please wait before retrying."}`; a valid admin Bearer exempt; `/api/health` and token-less requests never reach it. The plan asserts those values (see "Rate-limit pin"). The spec gap stands for the spec text: a later change to the limiter would break the pin without any spec line to say it should. Fix: cite the limiter's numbers in the spec or accept the pin as the oracle.
- **G3. The origin lock's refusal is unspecified except in First run.** R-23 says "the origin lock's refusal"; First run says "the origin lock's 404". The shipped code answers (review, `origin_lock.rs`) 404 with an empty body. The plan asserts 404, empty body, and equality of `(status, body, content-type)` with an unrelated path's. Fix: pin status and body in the spec.
- **G4. Log observation surface is unspecified.** R-18 and R-19 require an `error` line and a start line naming the disabled set and `deleted`, `absent` or `failed`, but give no format, stream or fields. The shipped code answers (review) that every harness start already logs error lines (`check_csp_meta`; `load_csp_header` when `csp-header.txt` is missing) and the tracing fmt layer timestamps each line, so a raw line diff is vacuous. The plan therefore requires an error line naming the switch or bad token, plus a normalised `(level, message, fields)` baseline diff. R-30's summary line has no binary-observable case. Fix: state the logging format and stream, and the start line's fields.
- **G5. Binary-observable limit of R-5's spawned-binary test.** A start function that awaited the first refresh would pass LL-PY-1 whenever the real sources answer inside 15 s (carried DD-review G-6). The in-process test with a never-answering source holds it; this plan does not claim it.
- **G6. `check-projects` after retirement.** R-29 retires it but gives no exit code or message. The shipped (pre-merge-2) subcommand exits 0 on a fresh valid list file and non-zero on a stale or missing one (the old spec's contract, from the blog-move harness header's read). Fix: say whether it is an unknown subcommand and name the exit code. LL-PY-14 asserts non-zero only, against a fresh file.
- **G7. "`/projects.json` no longer special" has no positive definition.** R-29 retires the file and its no-cache rule, but what the path returns afterwards is unstated. The shipped code answers (review, `static_files.rs:205`) that an absent static path falls to the SPA fallback. LL-PY-15 and LL-PY-16 assert status, `Content-Type`, `Cache-Control` (and, absent, the body) equal a control's, and no `no-cache`; other leftovers are undetected. Fix: state the expected response for the present and absent cases.
- **G8. The spec's Constraints names no edge-test form.** The form here is plan D4's. Fix: add a Constraints line.
- **G9. R-24's 15 s abort is observable in a browser only through a controlled clock, and the abort must use a timer the clock controls.** The spec does not say a browser test may fake time or which timer the page uses. The shipped tooling answers (review): Playwright 1.58.2's `page.clock` does not fake `AbortSignal.timeout`. LL-PW-4 states the precondition on T12 and a stated ~16 s real-wait fallback. Fix: allow clock control in R-24's acceptance and name the timer.
- **G10. R-29's "startup refusal on a missing or invalid list" has no stated acceptance for "invalid".** R-29's acceptance names only no list file and an empty store. LL-PY-17 (stale) and LL-PY-18 (malformed) are derived from the clause's text. Fix: add them to R-29's acceptance.

## Not covered by edge tests

- **R-1 to R-4, R-6 to R-15, R-17, R-30 (content)**: need a fake or hostile source. Held by in-process tests. Not claimed here.
- **R-16's "sources failing" and "sources hanging" variants and R-17's six-hourly tick**: need to control the sources and the clock. LL-PY-11 covers the part that does not, LL-PY-1 the 15 s health answer.
- **R-19's "unset and restart refills"** and the "behave normally" clauses: source-dependent.
- **R-20 (both prompts)**: reached only through the AI endpoints, which need a model credential and an outbound call. Unit tests.
- **R-31 to R-35 (Node checks, workflow assertions) and R-36 (audit)**: not binary or browser surfaces (D7).
- **First-real-run items**: production only, T18.
- **`fly.toml` has no `release_command`, the old spec's superseded line, the `git grep` retirements**: repository greps in task acceptance.
