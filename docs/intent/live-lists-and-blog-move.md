---
spec_type: code
frame_relevant: true
status: locked
lock_tag: brief/live-lists-and-blog-move/intent-locked
---

# Intake: live projects and articles lists, and the blog move to mnemra.dev

**Date:** 2026-11-18 · **Status:** locked (Peter, 2026-11-18: "Confirm and lock") · **Task:** #4853 · **Path:** full (all eight read-back questions answered)
**Consumer:** mixed. Visitors, recruiters especially, read /projects and use the chat; the site's AI reads both lists; downstream agents build from this doc.
**Kind:** server. The folio server calls two public sources at runtime and serves the site's pages.
**Reworks:** the projects intake `docs/intent/projects-from-github.md` and its Frame `docs/intent/projects-from-github-frame.md` (task #4726). See `## Conflicts with prior records`.

## Read-back

### RB-0 — Peter's rulings, 2026-11-18 (verbatim)

- "lets get the blog to point to mnemra.dev's blog page next."
- Scope pick: "link and redirect and open a task on mnemra.dev to redeploy the existing article as part of the mnemra.dev blog."
- "it will need to review and rework the article as well." This is tracked on mnemra.dev as task #4852.
- "tbh i would remove the /articles endpoint"
- "update the chat to pull the article list from mnemra.dev on deploy similar to the projects."
- "if you can do it live even better." Then: "agree live with a cache."
- "any reason to not make this a common one with projects and also it is live with a cache?"
- "doesn't have to be an hourly update , could be daily as well."
- "do you really need a secret to read a public repo". The lead measured 2026-11-18 that unauthenticated `GET https://api.github.com/orgs/mnemra/repos` returns 200 with `x-ratelimit-limit: 60`.
- "both in one brief"
- "also ensure mnemra.dev has a task to understand there is a dependency on its blog". Filed as mnemra.dev task #4858.
- "yes, read-back is correct"
- On full text versus summaries: "summaries only"

### RB-1 — Read-back, as confirmed ("yes, read-back is correct", 2026-11-18)

1. **Job.** folio's projects list and its AI chat stay current with what is published on GitHub and on mnemra.dev's blog, without a folio deploy. folio's own blog moves to mnemra.dev.
2. **Kind.** The folio server (Rust, Axum, on Fly.io behind Cloudflare). It calls two public sources at runtime: the GitHub REST API with no token, and mnemra.dev's RSS feed at https://mnemra.dev/rss.xml. It also serves the site's pages (SvelteKit SPA).
3. **Earliest real run.** Production, at the merge. The first start fetches both lists live, and the Articles link and redirects go live. Nothing stays behind a toggle.
4. **Attended.** The first run is attended. After that, the daily refresh runs unattended.
5. **Must stay as it is.**
   - The /projects page and its content rules: which repos are listed, description validation, grouping.
   - Bad data is never shown.
   - The chat otherwise behaves as today.
   - The admin login.
6. **Attackers.** The five in-scope actors Peter signed on 2026-11-18 for the projects Frame (ACTOR-1 anonymous visitor, ACTOR-2 anonymous GitHub user or PR author, ACTOR-4 holder of Peter's GitHub credentials, ACTOR-5 supply chain, ACTOR-6 Peter by mistake). One new path: text from mnemra.dev's feed enters the chat prompt at runtime. Its author is Peter. A compromised mnemra.dev host is a platform actor, which the signed list puts out of scope.
7. **Outside systems.** The GitHub REST API and mnemra.dev's RSS feed. Neither has an official Rust SDK, so both are plain HTTPS plus parsing.
8. **Promise words.**
   - "live": a change on GitHub or a new mnemra.dev post shows on folio within 24 hours, or at the next cold start, with no folio deploy.
   - "cache": a failed or invalid fetch keeps the last good copy.
   - "no data yet": when no good copy has ever existed, the chat runs without that list, and the projects page shows a plain unavailable state.

### RB-2 — Settled in the read-back

- The admin article editor goes, because the chat no longer reads folio's `articles` table. The table itself stays untouched, with no migration, as the projects table did.
- The chat gets title, summary, link and date per post from the feed. No full text.

### RB-3 — How RB-1 item 6 reads after review

RB-1 above is the confirmed text and is not edited. Two of its clauses are narrower than the record:
- "Its author is Peter." Feed text and repo descriptions are also written by whoever holds Peter's GitHub credentials (ACTOR-4; its widening to mnemra.dev's credentials is FQ-3), and by Peter acting by mistake (ACTOR-6). Both are in scope; see `## Security implications (draft scope)`.
- "A compromised mnemra.dev host is a platform actor, which the signed list puts out of scope." The 2026-11-18 list's ACTOR-7 does not name mnemra.dev's operator, and the signatures were given for the deploy-time design. Extending the list is Peter's to sign at the Frame (FQ-3, FQ-6).

## JTBD

**JTBD-1.** What folio says about Peter's work, on /projects and through its AI chat and job-fit analysis, has to keep up with what he publishes on GitHub and on mnemra.dev's blog, without him deploying folio each time. A visitor looking for Peter's writing has to land on the one place it lives, mnemra.dev.

## Non-goals

- **NG-1 — Full article text.** The chat gets no article body ("summaries only"; RB-2).
- **NG-2 — Changing folio's `articles` table.** The table and its one row stay untouched; no migration alters or drops it (RB-2). A small migration for the new last-good-copy store is allowed (OQ-7, ruled).
- **NG-3 — Articles outside mnemra.dev's feed.** The article list is what https://mnemra.dev/rss.xml carries, and nothing else.
- **NG-4 — Rewriting the article.** Reviewing, reworking and republishing "The Workflow Didn't Change" is mnemra.dev task #4852. Its full text is already exported (EV-7).
- **NG-5 — Changing mnemra.dev.** Its blog, its feed and the record of folio's dependency on them are mnemra.dev's; the dependency is task #4858.
- **NG-6 — Changing the /projects content rules.** Which repos are listed, description validation and grouping stay as they are (RB-1 item 5).
- **NG-7 — Changing the admin login** (RB-1 item 5).
- **NG-8 — Any other chat behaviour.** Beyond where the two lists come from and the move from full text to summaries, the chat behaves as today (RB-1 item 5).
- **NG-9 — A token or other secret for GitHub** ("do you really need a secret to read a public repo"; RB-1 item 2: "with no token"). Breaking this needs Peter first (FQ-2).
- **NG-10 — A toggle.** Nothing ships dark; both lists, the Articles link and the redirects are live at the merge (RB-1 item 3).
- **NG-11 — Repointing article URLs later.** No per-article redirect target, now or after #4852 republishes the article (OQ-3, ruled).
- **NG-12 — A staleness notice to visitors.** No page or chat answer tells a visitor a list is stale (OQ-6, ruled). The refresh timestamps of HC-15 are an operator signal, not a visitor notice.
- **NG-13 — A refresh endpoint.** No request handler starts a fetch (HC-18). A request that wakes a stopped machine leads only to that start's refresh.
- **NG-14 — Monitoring mnemra.dev's domain.** Peter: "i manage the domain separately, so its my problem to manage" (RL-2).

## Success criteria

Each criterion is tagged by where it is observed. **[P]** is observed in production after the merge. **[T]** is observed in tests, with both sources replaced through the fetch seam (HC-16). Prompt content is observed in tests as the output of `build_system_prompt` and `build_fit_prompt` (`libs/site-core/ai/context.rs`), never in production.

- **SC-1 — Projects stay current without a deploy. [P]** Measured with two reads, because a read that wakes a stopped machine sees the held copy before that start's refresh lands (HC-1). While GitHub is reachable and returns valid data, and with no folio deploy, a description changes on GitHub, or a new public repo that the content rules list appears. A first read of the HC-15 timestamps, made 24 hours or more after the change, wakes the machine if it is stopped. A second read, made once HC-18's refresh bound has passed since the first, returns a projects time later than the change, and a read of /projects after it shows the change.
- **SC-2 — Articles stay current without a deploy. [T] [P]** [T] After one refresh against a feed holding a new valid post, both prompts list it, and the feed's last-successful-refresh time is that refresh's. [P] Measured against SC-20's 48-hour bound, not the 24-hour promise, because the article list is visible only in the prompt: while the feed is reachable and valid, with no folio deploy, SC-20's second read passes on the feed's time.
- **SC-3 — The prompts' article entries. [T]** Each article entry in both prompts is one line carrying the post's title, summary, link and date from the feed (HC-20). Neither prompt holds article body text: the `content` of the row in folio's `articles` table does not appear in either.
- **SC-4 — The prompts' project entries. [T]** For one held copy of the projects list, the chat prompt, the fit prompt and the list /projects receives show the same repos and descriptions (projects intake success criterion 3; OQ-5, ruled).
- **SC-5 — A bad fetch keeps the last good copy, however old. [T]** Each case starts from a held good copy. A projects fetch that fails, holds any repo today's content rules reject, or leaves an owner with zero repos after R-3 (projects spec R-2) leaves the held projects copy in place, as D-4 of the projects Frame fails closed today. A feed fetch that fails, is unreadable, is empty, or holds no valid post leaves the held article copy in place (FQ-5 and OQ-9, ruled). Each case seeds a rejected payload and asserts the previous copy is still what /projects and both prompts receive, and that no rejected data appears in any of them (WS-7). A held copy is kept whatever its age (OQ-6, ruled).
- **SC-6 — No data yet. [T]** With no held copy of a list and every fetch of it failing, the chat and fit still answer, without that list. /projects then shows a one-line note that the project list is temporarily unavailable, with links to https://github.com/mnemra and https://github.com/prmanahan (OQ-1, ruled).
- **SC-7 — Bad data is never shown. [T]** No repo entry that today's content rules reject reaches /projects or either prompt; that validation runs on the live fetch (OQ-2, ruled). A feed post that breaks HC-14 is left out of both prompts and logged, and the feed's valid posts still update (FQ-5, ruled). A feed whose posts are all invalid keeps the held article copy (OQ-9, ruled).
- **SC-8 — /projects content rules unchanged. [T]** For the same GitHub data, /projects lists the same repos, with the same description handling and the same grouping and order, as the locked projects spec requires: R-3, R-4 and R-8 of `docs/specs/2026-11-11-projects-from-github.md`.
- **SC-9 — No link to folio's article pages. [T]** `git grep -nE "['\"\`]/(admin/)?articles" -- frontend/src ':!frontend/src/**/__tests__/**'` returns no line, and no test under `frontend/src` or `e2e/` asserts an `href` of `/articles`. The nav's and homepage's Articles links go to https://mnemra.dev/blog/. Known hits at `55159f6` are listed in EV-9.
- **SC-10 — folio's article URLs redirect permanently. [P] [T]** The paths that 301 are exactly: `/articles`, `/articles/`, and every path whose percent-decoded form starts with `/articles/` (so `/articles/x` and `/articles%2Fx` both 301). Paths outside that set, such as `//articles`, are FQ-14. [P] A request to `/articles` gets a 301 to https://mnemra.dev/blog/ with a bounded `Cache-Control`, not folio's SPA page (OQ-3 and OQ-4, ruled). [T] For `/articles`, `/articles/`, `/articles/x`, `/articles%2Fx` and `/articles?next=https://example.com/`, each under a forged `Host` and each `X-Forwarded-*` header, the response is a 301 whose `Location` is exactly https://mnemra.dev/blog/ (HC-13).
- **SC-11 — The article pages, the article APIs and the admin editor are gone. [P]** `/api/articles`, `/api/articles/{slug}` and every route under `/api/admin/articles` return the same JSON 404 as any unknown `/api` path (`api_not_found`, EV-6). The admin UI has no articles page, link or count (HC-7). folio has no `/articles` page routes.
- **SC-12 — The `articles` table is untouched. [T]** No migration added by the change alters or drops the `articles` table, and its schema and row are as before.
- **SC-13 — No secret for GitHub. [T] [P]** [T] No request the server sends to the GitHub REST API carries an `Authorization` header. [P] No GitHub token is set among the app's Fly secrets.
- **SC-14 — Unattended refresh. [P]** After the attended first run, while both sources are reachable and valid, two of SC-20's second reads, made more than 24 hours apart, show both lists' last-successful-refresh times (HC-15) advanced between them, with no human action.
- **SC-15 — Admin login unchanged. [T]** The existing admin login tests pass unchanged.
- **SC-16 — The last good copy survives a stop. [T]** After a restart with both sources failing, /projects and both prompts receive the copy held before the restart, from the first request on, and the timestamp read of HC-15 returns the times held before the restart (OQ-7, ruled). A held copy that fails validation on load is treated as no data yet, and its time is reported as missing (SC-6; HC-19).
- **SC-17 — A failed refresh is logged. [T]** Every refresh of either list that fails or is rejected logs a warning naming the list, and every feed post left out under HC-14 is logged (OQ-6 and FQ-5, ruled).
- **SC-18 — The deploy-time apparatus is gone. [T]** `git grep -nE 'projects\.json|fetch-projects|check-projects' -- ':!docs'` returns no line. At `55159f6` it returns 124 lines in 26 files (EV-9). `fly.toml` runs no `check-projects` release command, and the server starts with no list file present (OQ-2, ruled).
- **SC-19 — The deploy workflow's controls stay. [T]** Every assertion in `frontend/src/lib/__tests__/deploy-workflow.test.ts` at `e727c70` that does not name the fetch step survives, with its seeded violations still failing it. That includes, by its test names: `permissions` exactly `contents: read` and no job setting `permissions`; no workflow- or job-level env naming `FLY_API_TOKEN`; the Deploy `run:` exactly `flyctl deploy --remote-only`; every `uses:` pinned by SHA against the allowlist; the setup-flyctl version an exact `x.y.z`; `persist-credentials: false` on every checkout; and the files checked equalling the directory listing of `.github/workflows`. The secret rule becomes: no secret is referenced outside the Deploy step (the `GITHUB_TOKEN`-on-fetch exception goes). An assertion that names the fetch step together with another control keeps that other control: the Deploy step's `env` is exactly `FLY_API_TOKEN`. The `permissions` and secret assertions read `deploy.yml` only, so SC-20's workflow file is outside them and is held by SC-21 instead; it is inside the pin and checkout controls. Controls that count sites (`uses:` lines, checkout jobs) are updated to include SC-20's workflow file, never dropped, so that file sits inside the pin and checkout controls (HC-22). Stops ACTOR-5 (Cerberus round 2, rated low/medium, below the bar).
- **SC-20 — Staleness reaches Peter. [P] [T]** The check reads twice: a first read wakes a stopped machine, and a second read, made once HC-18's refresh bound has passed since the first, is the one judged. *The two reads are the architect's refinement within RL-1, so that a correct system on a machine that scales to zero (`fly.toml:9,11`) never reads red.* [T] The check fails red when the second read does not answer 200, when either list's time is missing or does not parse, or when either is more than 48 hours older than the check's clock; it passes when both are present and within 48 hours. Each of those five cases has a test. The timestamp response carries `Cache-Control: no-store`, also tested. [P] The check runs daily as a GitHub Actions job against production, with no secret (RL-1). That the job keeps running is FQ-11's known gap.
- **SC-21 — The daily check's job holds least authority. [T]** The R-7 workflow test asserts that SC-20's job declares `permissions` exactly `contents: read` and references no secret. A seeded copy with broadened `permissions`, a second `permissions` key, or a secret referenced anywhere in that job fails each assertion (D-C). Stops ACTOR-5 and ACTOR-4.

## Hard constraints

Where the review named the actor a constraint stops, the line says so.

- **HC-1 — Live with a cache.** Both lists are fetched by the folio server at runtime, and a failed or invalid fetch keeps the last good copy ("agree live with a cache"; RB-1 items 2 and 8). The refresh cadence must meet the 24-hour "live" promise; it need not be hourly ("doesn't have to be an hourly update , could be daily as well"). The bound on time-to-refresh is those 24 hours while the source is up; retry spacing after a failure is FQ-7. In a long outage the cache wins: the last good copy is served however old, and each failed refresh logs a warning (OQ-6, ruled). The last good copy persists in SQLite on the data volume, and the first request after a cold start gets it, before that start's refresh lands (OQ-7, ruled).
- **HC-2 — Two sources, fixed.** Unauthenticated, exactly these three URLs (RB-1 items 2 and 7; EV-3):
  - `https://api.github.com/orgs/mnemra/repos?type=public&per_page=100`
  - `https://api.github.com/users/prmanahan/repos?type=owner&per_page=100`
  - `https://mnemra.dev/rss.xml`

  Pagination builds page N on the fixed GitHub URL by adding `&page=N`; page N+1 is requested only while page N's `Link` header has `rel="next"`, and the URL inside that header is never requested. *Architect's decision (the round-2 fold asked for one of two forms), anchored to P-SecurityLayered: "a control that fails open (does not deny on guard error, or does not enumerate the permitted) is a defect". Building on the fixed URL keeps HC-17's permitted set to these three.* Rejected: following the server-supplied `next` after checking its scheme, host and path, which admits URLs the code did not enumerate. Neither source ships an official Rust SDK; both are plain HTTPS plus parsing.
- **HC-3 — Summaries only.** Title, summary, link and date per post; no full text (RB-2).
- **HC-4 — One brief.** Both lists and the blog move are designed together ("both in one brief").
- **HC-5 — No toggle.** Everything goes live at the merge, with an attended first run (RB-1 items 3 and 4).
- **HC-6 — Must stay as it is.** The /projects content rules, "bad data is never shown", the rest of the chat's behaviour, and the admin login (RB-1 item 5).
- **HC-7 — What goes, and what stays.** The `articles` table stays, unmigrated. The admin article editor, the `/articles` pages and the public `/api/articles` API go (RB-2; OQ-4, ruled). The admin dashboard's articles count and links go with the editor. *Decision, not a Peter ruling: anchored to the projects Frame's D-3, where Peter's "Remove the editor" also removed the dashboard's projects count, and to V-Consistency: "The same problem gets the same shape — within a surface, across surfaces, and across projects."* A new store for the last good copy may be added by migration (OQ-7, ruled).
- **HC-8 — The chat and fit AI paths stay asymmetric.** Chat streams through `libs/site-core/ai/anthropic_stream.rs` and fit stays on rig with `libs/site-core/ai/stop_reason.rs` (project `CLAUDE.md`, spec #572).
- **HC-9 — No compile-time embedding** (`decisions/DEFAULTS.md` G-0002; project `CLAUDE.md` guardrails).
- **HC-10 — Deploys stay on Fly.io through `.github/workflows/deploy.yml` on a merge to `main`** (project `CLAUDE.md` guardrails).
- **HC-11 — The deploy-time apparatus is retired.** The CI fetch step, the release command `check-projects` and the startup refusal go; validation moves into the live fetch (OQ-2, ruled). The static `/projects.json` file and its no-cache rule (projects spec R-11) go with them. /projects gets its list from the server's held copy; what serves it is FQ-8.
- **HC-12 — Chat and fit both get the live lists, through the shared prompt builder** (OQ-5, ruled). Both already read `build_base_prompt` today while their model call paths differ (EV-2, HC-8), so this leaves HC-8 as it is.
- **HC-13 — One redirect target, held as a constant.** `/articles`, `/articles/`, and every path whose percent-decoded form starts with `/articles/`, redirect permanently to https://mnemra.dev/blog/ (OQ-3 and OQ-4, ruled; SC-10; other forms are FQ-14). Nothing from the request path, query, `Host` or `X-Forwarded-*` reaches `Location`. The 301 carries a bounded `Cache-Control`. `/articles` paths leave `page_hits::should_track` (`libs/site-core/middleware/page_hits.rs:11-13`). (Warden WS-8.)
- **HC-14 — Feed validation: "Strict, skip bad posts"** (FQ-5, ruled). Per post: title at most 200 code points; summary at most 600 code points; plain text only, and a post containing markup is rejected, never stripped. Title and summary also reject the projects spec R-4's rejected class: control characters, invisible characters and line separators (D-A). The link is checked from a parsed URL, never a string prefix: scheme `https`, host exactly `mnemra.dev`, no userinfo, no port, normalized path under `/blog/`, no query, no fragment (Warden WS-5). A bad post is left out and logged; the good posts still update. An empty or unreadable feed, or one in which every post is bad, keeps the last good copy (OQ-9, ruled).
- **HC-15 — Staleness signal: "Daily CI check emails you"** (RL-1). The server exposes each list's last-successful-refresh time, timestamps only, no content, with `Cache-Control: no-store`, so an edge-cached answer cannot report a false green (the class the projects spec's R-11 guarded). Each time is persisted with its list's copy and loaded with it (HC-19). What advances it is D-B. A daily GitHub Actions job reads twice, waking the machine and then judging a second read after HC-18's refresh bound (SC-20), and fails red when that read does not answer 200, when a time is missing or does not parse, or when either is older than 48 hours, and GitHub emails Peter. No secret.
- **HC-16 — An injectable fetch seam.** Both sources are reached through one seam that tests replace, so every [T] criterion runs with no network (Cerberus H4).
- **HC-17 — The outbound client.** HTTPS only, TLS verified, redirects refused, and only the three fixed URLs of HC-2 called, GitHub pages included (`&page=N` on the fixed URL); each property has a test. Stops ACTOR-8, whose out-of-scope rating depends on these (Warden WS-2; FQ-4).
- **HC-18 — Bounded fetches, off the critical path.** A response body byte cap; connect and total timeouts per request; a feed item cap; a GitHub `User-Agent`. A **refresh bound**: an upper limit on one whole refresh of both lists, every page included, after which the refresh has either landed or failed; the spec sets its value, and SC-1 and SC-20 wait it. XML parsing refuses DTDs and entity expansion. Pagination, as HC-2 builds it, stops at a page cap, and a fetch that hits the cap or is truncated is rejected, so a partial list never replaces a good one. Fetches run off the request and startup critical path, so the server answers its health check within the 15-second grace period (`fly.toml:24`) whatever the sources do. No request handler starts a fetch (NG-13); a wake from stopped leads only to that start's refresh, whose GitHub cost is ACTOR-1's delta and FQ-2's budget. (Warden WS-3.)
- **HC-19 — The store.** Validate, then persist, then swap the in-memory copy, as one step that leaves either the old copy or the new one. The store holds only the typed, validated fields the lists use, never raw response bodies, plus each list's last-successful-refresh time (HC-15): one row per list, overwritten on success, no history. A held copy is validated again on load, and a copy that fails is treated as no data yet, with its time reported as missing. A break-glass route for a stored entry that is valid but wrong is written down before the first run; its content is FQ-9. (Cerberus M1, M5; Warden WS-6.) The persisted copy outliving a rollback is part of ACTOR-4's widened reach (`## Security implications`).
- **HC-20 — Data, not instruction, for the feed too.** The articles section of the prompt opens with a data-not-instruction preamble, as the projects section does (projects spec R-12; `SECTION_PROJECTS`, `libs/site-core/ai/prompt_templates.rs:59-60`). Today `SECTION_ARTICLES` (`:61`) is a bare heading. Each article entry is one prompt line, as R-12 requires of each repo (D-A). Composed links (R-13) and text-only rendering (R-14) stay. (Cerberus M6; Warden WS-4.)
- **HC-21 — New dependencies are a level-3 decision.** A direct HTTP client and an XML or RSS parser in the app process are new ACTOR-5 entries; the choice is put to Peter at the Frame or spec (FQ-10).
- **HC-22 — The deploy workflow's controls stay.** Retiring the fetch step keeps every projects-spec R-7 control that does not name it (SC-19). The pin and checkout controls cover every workflow file, SC-20's included; the `permissions` and secret controls read `deploy.yml` only, and SC-20's job is held by D-C instead (SC-21). Stops ACTOR-5. (Cerberus B2, rounds 2 and 3.)

## Evidence

**EV-1 — Peter's rulings and the confirmed read-back**, 2026-11-18: RB-0, RB-1, RB-2 above. Inherited from the lead's source notes; the quotes are copied verbatim.

**EV-2 — Measured by the architect on `main` at `e727c70`, 2026-11-18:**
- The Articles links point at folio: `frontend/src/lib/components/Nav.svelte:10` (`href: '/articles'`), `frontend/src/lib/components/Hero.svelte:107` and `frontend/src/lib/components/HeroSkeleton.svelte:40` (`href="/articles"`).
- folio serves articles itself: the SvelteKit routes `frontend/src/routes/(public)/articles/+page.svelte` and `frontend/src/routes/(public)/articles/[slug]/+page.svelte`; the public API `/api/articles` and `/api/articles/{slug}` (`libs/site-core/routes/articles.rs:35-36`); the admin editor (`libs/site-core/routes/admin/articles.rs`, `frontend/src/routes/admin/articles/+page.svelte`).
- The prompt reads the `articles` table (`libs/site-core/ai/context.rs:45`) and appends each published article's full `content` (`libs/site-core/ai/context.rs:313-326`).
- Chat and fit share one base prompt: `build_system_prompt` and `build_fit_prompt` both call `build_base_prompt` with the projects list (`libs/site-core/ai/context.rs:342`, `:354`). Fit reads the articles and projects sections today.
- The projects list is deploy-time today: the CI step that writes `frontend/static/projects.json` (`.github/workflows/deploy.yml:112`), the fetch script `frontend/scripts/fetch-projects.mjs`, the release command `./folio check-projects` (`fly.toml:36`) with a 30-minute age bound (`libs/site-core/projects_list.rs:31`), and the server's refusal to start when the list is invalid or absent (`cmd/server/main.rs:85-89`).
- Unknown paths get the SPA page with 200: the static service "serves `index.html` with 200 so the SPA router can handle client-side routing (#4718)" (`libs/site-core/static_files.rs:196-197`). Removing a SvelteKit route alone therefore yields a 200 page, not a redirect.
- Machines stop when idle: `auto_stop_machines = "stop"` and `min_machines_running = 0` (`fly.toml:9`, `fly.toml:11`).

**EV-3 — Inherited from the lead, 2026-11-18, not re-run here:**
- Unauthenticated GitHub requests per refresh: 2. `orgs/mnemra/repos?type=public&per_page=100` and `users/prmanahan/repos?type=owner&per_page=100` each returned 200 with no `Link` header, so each is one page. The unauthenticated limit is 60 per hour per IP. Measured from the lead's machine, not from Fly.
- mnemra.dev's blog is at https://mnemra.dev/blog/; its feed carries title, link, description (the summary) and pubDate per item, and lists one post, hello-mnemra.
- The `articles` table holds one published article, "The Workflow Didn't Change" (production database; not in the repository).
- mnemra.dev hosting: served by Cloudflare (`server: cloudflare`); nameservers elias/katelyn.ns.cloudflare.com; registrar CloudFlare, Inc. (RDAP); registered 2026-04-22, expires 2027-04-22T19:48:23Z.
- `reqwest` 0.13.2 is in the build only transitively, through rig-core.

**EV-4 — Prior record.** The locked simplification Frame's L-10 (`docs/intent/folio-simplification-frame.md:43`, RULED) reads: "Articles live on mnemra.dev only. folio redirects its article URLs to `https://mnemra.dev/blog/`. folio's one article moves there (`#3559`). The AI sees articles as summaries and links only, never full text." (bold dropped). The same row carries a note it labels INHERITED, not Peter's words: "the redirect must not go live before the article exists on mnemra.dev". Peter rejected that note's premise on 2026-11-18 (OQ-3, ruled; CX-8).

**EV-5 — Peter's answers, 2026-11-18.** Relayed by the coordinator in two rounds; inherited, not heard by the architect. Recorded under `## Rulings`.

**EV-6 — Measured by the architect at `62a5a14`, 2026-11-18.** An unknown `/api` path answers with a JSON 404 from `api_not_found` (`libs/site-core/static_files.rs:187-189`, routed at `:211-213`), never the SPA page.

**EV-7 — The article's full text is exported.** `exports/folio/2026-11-18-the-workflow-didnt-change.json` in brain, commit `f3f9049`, taken from `https://peter.manahan.io/api/articles/the-workflow-didnt-change` before the endpoint is removed. The architect confirmed the file (18087 bytes) and the commit in brain on 2026-11-18. Closes Cerberus M4.

**EV-8 — Reviews, 2026-11-18.** Cerberus round 3 of `f1b13a6`: passed, zero new blocker or high findings, two text fixes and a carry-to-Frame list (FQ-15); inherited through the coordinator. Cerberus round 2 (an Opus run) of `cc9be38`: request changes, two new Highs (H-1, H-2), both from earlier fixes, plus Mediums and Lows; inherited through the coordinator's round-3 fold. Round 1, of `55159f6`: Cerberus (Stage 1c): request changes, B1, B2, H1 to H4, M1 to M6, L1, L2. Warden (`security-implications-checklist`): WS-1 to WS-9, nothing above the bar. Inherited through the coordinator's fold; the architect did not read either review in full.

**EV-9 — Measured by the architect at `55159f6`, 2026-11-18:**
- SC-18's search returns 124 lines in 26 files. Among them: `frontend/scripts/fetch-projects.mjs`, `.gitignore:22-24`, `README.md:41,44,64`, `CLAUDE.md:39`, `justfile:19-27`, `cmd/server/main.rs`, `cmd/server/tests/check_projects.rs`, `libs/site-core/static_files.rs:37` (`NO_CACHE_FILE`), `libs/site-core/tests/test_projects_json_cache.rs`, `frontend/src/routes/(public)/projects/+page.svelte:51`.
- SC-9's search hits, outside tests: `frontend/src/lib/components/Nav.svelte:10`, `Hero.svelte:107`, `HeroSkeleton.svelte:40`, `ArticleCard.svelte:7`, `frontend/src/routes/(public)/+page.svelte:49`, `frontend/src/routes/(public)/articles/[slug]/+page.svelte:30`, `frontend/src/lib/api.ts:18-19`, `frontend/src/lib/admin-api.ts:125-129`, and the admin layout and page lines in the next bullet. Tests asserting `href` `/articles`: `e2e/accessibility.spec.ts:78`, `frontend/src/lib/__tests__/Hero.test.ts:71`, `frontend/src/lib/__tests__/Header.test.ts:89`; `e2e/public-pages.spec.ts` and `e2e/responsive.spec.ts` navigate to it.
- The admin dashboard's articles count and links: `frontend/src/routes/admin/+layout.svelte:32,44`, `frontend/src/routes/admin/+page.svelte:13,22`, `libs/site-core/routes/admin/dashboard.rs:11,31,44`.
- `page_hits::should_track` tracks `/articles` and `/articles/*` (`libs/site-core/middleware/page_hits.rs:11-13`).
- The projects section carries the data-not-instruction preamble; the articles section does not (`libs/site-core/ai/prompt_templates.rs:59-61`).
- The health-check grace period is `"15s"` (`fly.toml:24`).

## Risk profile

**May touch trust boundary: yes.** Recorded as a required Stage-2 constraint: Warden's security-mode review runs at the Frame.
- [x] **New runtime outbound fetches.** The folio server calls the GitHub REST API and https://mnemra.dev/rss.xml at runtime. Today the server makes no outbound request for either list.
- [x] **Feed text into the prompt.** Title, summary, link and date from mnemra.dev's feed enter the chat and fit prompts at runtime (RB-1 item 6; OQ-5, ruled).

Also new: a store of fetched text in SQLite on the data volume, served at start (HC-19); a public read of the two refresh timestamps (HC-15); and new crates in the app process (HC-21). The AI prompt surface has no review lens (FQ-13).

## Security implications (draft scope)

Warden's discovery-stage read ran on 2026-11-18 (`security-implications-checklist`): WS-1 to WS-9, nothing above the bar. Its findings are carried as HC-13, HC-14, HC-17 to HC-21 and SC-5, SC-10, SC-11. This section copies what the coordinator relayed from Warden's report; the asset list is quoted from it.

**Deployment.** The folio server on Fly.io behind Cloudflare, as in the projects Frame's `### Deployment`. New for this feature: outbound HTTPS from the Fly machine to api.github.com (2 unauthenticated requests per refresh, EV-3) and to mnemra.dev; the SQLite store of HC-19; the timestamp read of HC-15. mnemra.dev is served by Cloudflare, with Cloudflare nameservers and registrar (RL-2).

**Assets and impact** (paraphrased from Warden's discovery read, 2026-11-18, as relayed by the coordinator; text in double quotes is Warden's exact wording; the `AS-n` ids and the bracketed cross-references are the architect's):
- **AS-1 — Chat and fit reply integrity** (visitors read model output that quotes feed and repo text): **medium**. Text can steer the AI, and replies can carry a clickable link to any http(s) host (KG-6, TH-12) [both in the projects Frame].
- **AS-2 — The `/projects` page content:** **medium**.
- **AS-3 — The persisted last-good store on the volume**, which now survives deploys: **medium**. A poisoned or stale entry persists across restarts, where before a deploy replaced the list. [HC-19]
- **AS-4 — Availability of "a 256mb single machine":** **medium**. The doc's own rating is "deploy path blocked for hours". [The rating is the projects Frame's bar, where "the deploy path blocked for hours" is medium impact.]
- **AS-5 — Anthropic API key, `ADMIN_PASSWORD` and `EDGE_AUTH_TOKEN`**, in the same process as the new parser: **high**, unchanged by this feature. [HC-21]
- **AS-6 — The `articles` table row:** **low**. It's dead data after the change (NG-2), so it's an exposure with no reader.

**Actor deltas against the 2026-11-18 list**, rated on its bar (Cerberus H1):
- ACTOR-1 — Anonymous visitor — in — each visit can wake the machine, and each wake spends GitHub budget — medium/low.
- ACTOR-2 — Anonymous GitHub user — in — the owner-name takeover (projects Frame KG-5) is now exposed continuously, not only at deploy; R-4's owner check stays — low/medium.
- ACTOR-4 — Holder of Peter's credentials — in — gains capability: feed text or repo descriptions now steer the public chat with no folio deploy, and the persisted copy survives a rollback — low/medium. Widening it to mnemra.dev's host, registrar, DNS and deploy credentials is FQ-3.
- ACTOR-5 — Supply chain — in — new direct crates in the app process (HC-21, FQ-10) — no cell given; the Frame rates and re-signs it under FQ-6.
- ACTOR-6 — Peter by mistake — in — writes feed and description text (RB-3) — no cell given; the Frame rates and re-signs it under FQ-6.
- mnemra.dev domain lapse — out — Peter manages the domain separately: "i manage the domain separately, so its my problem to manage" — retires: none.

The actor set, bar and known-gaps list are re-signed for the live design at the Frame (FQ-6).

## Open questions

None open. OQ-1 to OQ-7 and OQ-9 are ruled; OQ-8 became FQ-5, also ruled. See `## Rulings`.

## Rulings

Peter's answers, 2026-11-18, as relayed by the coordinator (EV-5). Each entry keeps the question, the ruling with his option or words in quotes, and what carries it. The ids are kept so earlier references still resolve.

- **OQ-1 — The projects page with no data yet. Ruled: "Note plus GitHub links".** The page shows a one-line note that the project list is temporarily unavailable, plus links to github.com/mnemra and github.com/prmanahan. Refines RB-1 item 8's "a plain unavailable state". Carried by SC-6.
- **OQ-2 — The deploy-time apparatus. Ruled: "Retire all three".** The CI fetch step, the release command `check-projects` (`fly.toml:36`) and the startup refusal (`cmd/server/main.rs:85-89`) are retired. Validation moves into the live fetch. Carried by HC-11, HC-22, SC-7, SC-18 and SC-19.
- **OQ-3 — The redirect target for `/articles/<slug>` until #4852 republishes the article. Ruled: premise rejected.** Peter, verbatim: "This doesn't make sense, folio is just pointing to the blog page directly. The only use of the rss feed is for the chat. so i don't understand why repointing is needed since it would point to the same page." `/articles` and every `/articles/*` path redirect permanently to https://mnemra.dev/blog/, with no per-article repointing later. Carried by HC-13, SC-10 and NG-11. Rejected alternative: L-10's inherited sequencing note, "the redirect must not go live before the article exists on mnemra.dev" (EV-4); it lost because the redirect never points at the article, so the article's existence on mnemra.dev does not bear on it (CX-8).
- **OQ-4 — Which "/articles endpoint" goes. Ruled: "Pages and API".** The `/articles` pages and the public `/api/articles`, `{slug}` included, are removed. The old page URLs 301 to the blog; the old API URLs get the standard JSON 404 (EV-6). The admin editor goes and the `articles` table stays untouched. Carried by HC-7, SC-10 and SC-11.
- **OQ-5 — Fit's prompt. Ruled: "Yes, both".** Chat and fit both get the live lists through the shared builder. Carried by HC-12, SC-3 and SC-4.
- **OQ-6 — "live" against "cache" under a long outage. Ruled: "Keep serving, log it".** The last good copy is served however old it is; a warning is logged on every failed refresh; visitors are not told. Carried by HC-1, SC-5, SC-17 and NG-12. RL-1 adds the operator signal.
- **OQ-7 — Does the last good copy survive a stop? Ruled: "Persist it".** The last good copy goes in SQLite on the data volume, is served at cold start, and is then refreshed. This needs a small migration: the `articles` table stays untouched, but a new store is allowed. Carried by HC-1, HC-7, HC-19, NG-2 and SC-16.
- **FQ-5 — What counts as bad feed data (formerly OQ-8). Ruled: "Strict, skip bad posts".** Per post: title at most 200 characters; summary at most 600 characters (counted in code points, D-A); plain text only, a post containing markup rejected, never stripped; the link must parse to https://mnemra.dev/blog/... exactly. A bad post is left out and logged. The good posts still update. An empty or unreadable feed keeps the last good copy. Rejected alternative: the lead's earlier default, under which any invalid entry rejected the whole fetch. Carried by HC-14, SC-5, SC-7 and SC-17; OQ-9 rules the case it left open.
- **OQ-9 — A feed whose posts are all invalid. Ruled: "feed case: keep last good".** A readable feed in which every post breaks HC-14 keeps the last good copy, as an empty or unreadable feed does, so a bad source never blanks the list. Rejected alternative: the article list becomes empty. Carried by HC-14, SC-5 and SC-7.
- **RL-1 — Staleness signal. Ruled: "Daily CI check emails you".** The server exposes each list's last-successful-refresh time, timestamps only, no content. A daily GitHub Actions job fails red if either is older than 48 hours, and GitHub then emails Peter. It uses existing infrastructure and needs no secret. Answers Cerberus H2 and the signal half of Warden WS-7. Carried by HC-15, SC-2, SC-14 and SC-20.
- **RL-2 — mnemra.dev hosting.** Peter: "i think its hosted by cloudflare", confirmed by the lead's measurement (EV-3). Peter: "i manage the domain separately, so its my problem to manage". No domain-expiry check in folio; the domain is an out-of-scope dependency Peter owns. Peter later clarified that his "don't bother having special code to check that" meant the domain end-date check only; the daily stale-list check (RL-1) stays. Answers Warden WS-1's hosting question. Carried by NG-14 and `## Security implications`.
- **#4837 — Ruled: "Hold #4837".** The projects Frame re-lock waits. This feature's Frame carries the threat model for the live design (CX-6, CX-7, FQ-6).

## Lead decisions

The lead's decisions at level 4: acted on and reported to Peter, who may overturn any of them in a word. They are not Peter's rulings.

- **D-A — Newlines and control characters in feed text.** Feed title and summary take the projects spec R-4's rejected class: control characters, invisible characters and line separators. Lengths are counted in code points. A post that breaks this is skipped and logged, the same disposition as markup under Peter's "Strict, skip bad posts". HC-20 carries R-12's one-line-per-entry rule for articles as well as the preamble. *Anchor: V-Consistency, "The same problem gets the same shape — within a surface, across surfaces, and across projects."* Carried by HC-14, HC-20 and SC-3.
- **D-B — What advances a last-successful-refresh time.** The lead's text, as amended by the lead on 2026-11-18: "It advances when the fetch succeeds: the source answered 200, parsed, and yielded at least one valid entry. Skipped bad posts do not stop it advancing." For projects, the whole list must also pass today's content rules (SC-5). It does not advance when the source is unreachable or unparseable, or yields no valid post (OQ-9: keep last good); the 48-hour check then fires. An owner with zero repos after R-3 is a failed fetch, per the projects spec's R-2: the last good copy stays and the time does not advance. A post skipped while the refresh still succeeds is traced only by its log line, a known gap (FQ-11) within Peter's "left out and logged". *Anchor: OQ-9's ruling (the lead's); and P-TrustworthySignal, "A verification signal means what it reports, or it is a defect to fix" (the architect's choice, since the lead gave none) — the time reports a refresh that delivered data, so it moves only when one did.* Carried by HC-15, SC-2, SC-5, SC-14 and SC-20. Rejected alternative: the lead's original clause "or a valid empty list for projects", which also advanced the time on an empty projects list. The lead dropped it on 2026-11-18 because it conflicted with R-2, under which a fetch where "an owner has zero repos after R-3" fails, and with OQ-9's "a bad source never blanks the list".
- **D-C — The daily check's job holds least authority.** SC-20's workflow job declares `permissions: contents: read` and references no secret, with a [T] assertion in the R-7 workflow test and a seeded violation for each. Without this, the R-7 `permissions` and secret controls, which read `deploy.yml` only, would leave the new job unchecked: a known gap to park, which this requirement closes. *Anchor: P-LeastAuthority, "A component, agent, or credential holds the minimum standing authority its role requires."* Stops ACTOR-5 and ACTOR-4. Carried by HC-22 and SC-21. (Cerberus round 3.)

## Carried to the Frame

Design items the Frame owes an answer on. They are not intake decisions.

- **FQ-1 — One mechanism or two.** Peter asked: "any reason to not make this a common one with projects and also it is live with a cache?" The Frame answers it with its reasons. The answer has to hold both validation rules as ruled: a projects fetch with any rejected repo is rejected whole (projects Frame D-4), while the feed skips bad posts (FQ-5).
- **FQ-2 — GitHub budget from Fly.** EV-3 measured 2 requests per refresh and 60 per hour per IP, from the lead's machine. The Frame measures the budget from Fly's egress, counting refreshes, cold starts (ACTOR-1's wakes) and retries. If the budget does not hold, the Frame escalates to Peter before any design breaks NG-9. (Cerberus H3.)
- **FQ-3 — mnemra.dev in the actor set.** ACTOR-7 would have to name mnemra.dev's operator (Cloudflare, RL-2), for Peter to sign at the Frame. ACTOR-4 would likewise widen to mnemra.dev's host, registrar, DNS and deploy credentials, also for his signature. The domain-lapse path stays out of scope, owned by Peter (RB-3).
- **FQ-4 — ACTOR-8 restated.** The projects Frame's ACTOR-8 covers "between the runner and api.github.com or Fly". It is restated to the Fly→api.github.com and Fly→mnemra.dev paths, and stays out of scope only while HC-17 holds.
- **FQ-6 — Re-sign for the live design.** The actor set, bar and known-gaps list. The 2026-11-18 signatures were given for the deploy-time design (CX-7).
- **FQ-7 — Retry after a failed refresh.** Spacing and count, inside HC-1's 24-hour bound and FQ-2's budget (Cerberus H4).
- **FQ-8 — What serves /projects its list.** The static `/projects.json` goes (HC-11), and SC-18 rules out the name. The route, its response shape and its caching are the Frame's; an edge-cached list is the class the projects spec's R-11 guarded and HC-15's `no-store` guards for the timestamps.
- **FQ-9 — The break-glass route.** How Peter removes a stored entry that is valid but wrong, given NG-13's no refresh endpoint (HC-19; Warden WS-6).
- **FQ-10 — The new crates.** The HTTP client and the XML or RSS parser, each a level-3 decision with its ACTOR-5 exposure (HC-21).
- **FQ-11 — Known gaps.** The projects Frame's KG-3 and KG-4 dissolve with the deploy-time design. KG-6 (text steering the AI) widens to the feed. Two more go on the Frame's known-gaps list:
  - **The daily check can stop silently.** GitHub may disable a scheduled workflow in a public repository after a stretch with no activity, and may delay or drop scheduled runs (Cerberus round 2, recalled from documentation, not verified). Nothing notices SC-20's job not running. Peter keeps the mechanism to existing infrastructure, with no external dead-man service, so this is parked: the Frame states the human cadence on which Peter confirms the job still runs.
  - **A skipped post is traced only by its log line** while the refresh still succeeds (D-B).
- **FQ-12 — Typed DFD.** Trust boundaries for the new flows, per Warden.
- **FQ-13 — The AI prompt surface is unlensed.** No AI or LLM review lens exists; the Frame records the ratings on it as structural.
- **FQ-14 — Article-like paths outside SC-10's set.** Paths such as `//articles` or `/./articles`, whose percent-decoded form is not `/articles`, `/articles/` or under `/articles/`, are not in the redirect set. Whether they 301 to the same constant or take the existing fallback is for the Frame or the spec. Whatever is chosen, any such path that 301s has the constant `Location` of HC-13.
- **FQ-15 — Cerberus round 3's carry-to-Frame list.** The intake states what each must achieve; the Frame or spec settles the detail:
  - HC-2's pagination detail, HC-13's percent-decoding, and the cache-header specifics.
  - SC-20's wait value between its two reads, and whether a failed first read is tolerated.
  - SC-19's per-test-name list.
  - HC-14's definition of markup for RSS, entities and CDATA included.
  - The refresh bound's value (HC-18) and how it interacts with FQ-7's retries.
  - A [T] case for a running server refreshing without a restart.
  - A number for HC-13's "bounded `Cache-Control`".
  - Whether the feed gets a 24-hour [P] check, beyond SC-2's 48-hour bound.
  - FQ-11's human cadence.

## Conflicts with prior records

What this intake changes in records that are still on `main` at `e727c70`. Recorded so this feature's Frame sees them. The projects re-lock, #4837, is held ("Hold #4837", Peter, 2026-11-18); see CX-6 and CX-7.

- **CX-1 — Projects intake, runtime calls.** `docs/intent/projects-from-github.md` lists "Calling GitHub per visitor or at runtime" as a non-goal and "No new server runtime surface" as a hard constraint. RB-1 item 2 reverses both.
- **CX-2 — Projects intake, failure behaviour.** That intake's promise word "build time" and its success criterion 5 fail the deploy rather than "shipping an empty or stale list". RB-1 item 8's "cache" keeps the last good copy instead.
- **CX-3 — Projects Frame, D-1 and QA-1, QA-2.** The CI fetch, the release-time age check and "the release never serves a list from an earlier run" are the deploy-time design this feature replaces.
- **CX-4 — Projects Frame, D-8.** D-8 fetched with `GITHUB_TOKEN` and rejected "No token" because "shared runner addresses risk rate-limit false reds". RB-1 item 2 adopts no token, from the server rather than a runner; FQ-2 measures the budget.
- **CX-5 — Projects Frame, D-9.** D-9's Node fetch script ran in CI; the fetch now runs in the Rust server.
- **CX-6 — Projects spec, R-10, R-11 and R-12.** R-10 refuses to start on an absent list, against RB-1 item 8. R-11 serves `/projects.json` with `no-cache`, which HC-11 retires. R-12 says "the server SHALL make no request to GitHub", against RB-1 item 2; its preamble survives and extends to articles (HC-20). R-10's refusal, with R-18 and R-19's release check, is retired (OQ-2, ruled). #4837, the projects Frame re-lock, is held ("Hold #4837"); this feature's Frame carries the threat model for the live design.
- **CX-7 — Projects Frame, signatures.** At `e727c70` the Frame's `### Signatures` says "None is written, so this scope is unsigned". RB-1 item 6 and the lead's notes say Peter signed on 2026-11-18 "for the deploy-time design", which this feature retires. With #4837 held, this feature's Frame starts from the 2026-11-18 attacker list, applies the deltas in `## Security implications`, and has Peter re-sign (FQ-6).
- **CX-8 — Simplification Frame, L-10 sequencing. Superseded 2026-11-18.** EV-4's inherited note held the redirect until the article exists on mnemra.dev; RB-1 item 3 puts it live at the merge. Peter rejected the note's premise (OQ-3, ruled): the redirect goes to the blog page, not to the article, so it goes live at the merge. The note stays on record as the rejected alternative. L-10's target, https://mnemra.dev/blog/, matches the ruling. L-10 names the article move as `#3559`; this feature's record names #4852.
- **CX-9 — Simplification Frame, L-4, and the content bundle.** The projects Frame records its departure from L-4 ("job is the source of truth for all content") as interim, parked on #3554. A runtime fetch from mnemra.dev widens that departure to articles. The parked bundle spec (`docs/specs/2026-09-14-content-bundle.md`) carries mnemra.dev article summaries and links through job's bundle, a different route.

## Consultations

- **Cerberus, Stage 1c review of `55159f6`, 2026-11-18.** Request changes: B1, B2, H1 to H4, M1 to M6, L1, L2. B1 closed by FQ-5's ruling; the rest folded into the fields named in each line above.
- **Cerberus round 3, review of `f1b13a6`, 2026-11-18.** Passed: zero new blocker or high findings. Two text fixes taken before the lock: HC-22 narrowed to the pin and checkout controls, with D-C and SC-21 added; FQ-3 reworded to a proposal for Peter to sign. Its carry-to-Frame list is FQ-15.
- **Cerberus round 2 (Opus), review of `cc9be38`, 2026-11-18.** Request changes: H-1 and H-2, plus Mediums and Lows. Folded into SC-1, SC-2, SC-10, SC-14, SC-16, SC-19, SC-20, HC-2, HC-13 to HC-15, HC-17 to HC-20, HC-22, NG-13, FQ-8, FQ-11, FQ-14, the asset labels and the lead decisions D-A and D-B.
- **Warden, discovery-stage read of `55159f6`, 2026-11-18.** WS-1 to WS-9, nothing above the bar; folded as listed under `## Security implications`.

## Dismissed review flags

- **L2 (Cerberus) — de-duplicate SC-14 against SC-1 and SC-2, SC-15 against NG-7 and HC-6, NG-12 against OQ-6.** Taken in part: SC-14 now observes the refresh timestamps in production, which SC-1 and SC-2 do not; NG-12 now separates the visitor notice from the operator signal. SC-15 is kept: NG-7 and HC-6 state the intent and SC-15 is its check. No id is removed, so earlier references still resolve.
