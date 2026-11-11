---
status: draft            # draft | locked; Puck stamps at spec-exit
lock_tag: ""             # brief/projects-from-github/spec-locked
---

# Feature: projects page from GitHub

> **Spec for:** folio task #4726 · **Date:** 2026-11-11
> **Design ref:** Frame `docs/intent/projects-from-github-frame.md` (draft, untagged, written against blob `c1ab9acc31f8642c4449cdfbf96136bbd1117380`) · intake `docs/intent/projects-from-github.md`

## Purpose
/projects and the site's AI list exactly the public repos of the GitHub owners `mnemra` and `prmanahan`, grouped by owner, with their GitHub descriptions. The deploy job fetches that list once per deploy into one file, `/projects.json`, which both read, so they cannot disagree. The database projects, Puck Playbook among them, leave every public and AI path. The Frame's D-1 to D-9 carry the anchors.

**Departure from /brief, by Peter's ruling (2026-11-11):** "Trim, then light review". The Frame threat-modeling pass, Wyrd's dry decompose and Charon's canon check do not run for this change.

## Requirements
**Fetch** (D-1, D-4, D-8, D-9)
- **R-1** A Node script SHALL fetch from the GitHub REST API the repos of `mnemra` and `prmanahan`, following pagination to the last page, and write the list file. It SHALL use only Node's built-in `fetch`, with no new dependency. It SHALL run as a new step in the `deploy` job of `.github/workflows/deploy.yml`, placed above `Deploy`. Acceptance: when a test double serves one owner across two pages, the file contains the repos from both pages.
- **R-2** The script SHALL first delete any file at its output path. It SHALL then exit non-zero, leaving no file, when any of these holds: a request fails or returns a non-2xx status (401 and 403 included); a response body does not parse as a repo array; pagination cannot complete; an owner has zero repos after R-3; an element fails R-4. It SHALL NOT substitute a fixture, a cached copy, a committed copy or an earlier list. Acceptance: one seeded case per condition, each starting with a valid file at the output path, exits non-zero and leaves no file there.
- **R-3** Every returned repo SHALL be listed unless `fork` is true, `archived` is true, or its name equals `.github` (case-insensitive). No other exclusion, allowlist or per-repo setting SHALL exist. Acceptance: a fixture repo whose name appears nowhere else in folio is listed; a fork, an archived repo and `.github` are not.
- **R-4** The script SHALL fail, never skip, on an element that meets any of these: `private` is not `false`; its owner login differs case-insensitively from the owner requested; its name does not match `^[A-Za-z0-9._-]{1,100}$`; its description is neither null nor a string. Acceptance: one seeded element per condition exits non-zero.
- **R-5** The script SHALL authenticate with the job's built-in `GITHUB_TOKEN`, read from the environment; `permissions` SHALL stay `contents: read`; no PAT or new secret SHALL be added; the token SHALL NOT appear in argv, in the file or in any log; `FLY_API_TOKEN` SHALL NOT be in the step's environment. Acceptance: a run with a sentinel token leaves the sentinel in neither the file nor captured stdout and stderr. The new step references no secret other than `GITHUB_TOKEN`.
- **R-6** For each owner, the script SHALL log the included names and their count, each excluded name with its reason, and each included name that has an empty description. On failure it SHALL log the condition that failed. Acceptance: for the shared API fixture, the captured output contains each of these lines.

**List file** (D-5, D-6, D-7)
- **R-7** The file SHALL hold exactly the fields in **Data Model**. Groups SHALL be ordered `mnemra`, then `prmanahan`. Repos SHALL be sorted by name, ascending and case-insensitive. Each `url` SHALL be composed as `https://github.com/<owner>/<name>`, and no other GitHub field SHALL be held. Acceptance: for the shared API fixture, the output equals a committed expected list file, apart from `fetched_at`.
- **R-8** The generated file SHALL NOT be tracked in git. Every committed list-file fixture SHALL use repo names that begin `fixture-`. API-response fixtures, which are the script's inputs, are exempt. Acceptance: after a run in a clean checkout, `git status --porcelain` prints nothing.

**Server** (D-2)
- **R-9** At startup the server SHALL read the list file from the static directory. It SHALL refuse to start, naming the path, when the file is absent, does not parse, violates **Data Model**, or has an owner with zero repos. It SHALL log `source`, `fetched_at` and the count per owner. Acceptance: one spawned-binary case per condition, plus a case showing that R-7's committed expected list file loads.
- **R-10** The AI prompt's projects section (`libs/site-core/ai/context.rs`) SHALL be built from the loaded list only, giving owner, name, `url`, and the description when it is non-empty. The section SHALL stay in its current position after the security block. It SHALL NOT read the `projects` table, and the server SHALL make no request to GitHub. Acceptance: with the table holding a published `Puck Playbook` row and the expected list file loaded, the prompt contains every fixture repo name and does not contain `Puck Playbook`.

**Page** (D-2, D-5, D-7; V-Accessibility)
- **R-11** `/projects` SHALL load `/projects.json` at runtime and render: one group per owner, in file order, under a heading one level below the `h1` that names the owner; one entry per repo, in file order, showing its name as a link to `url` (`target="_blank" rel="noopener noreferrer"`) and then its description. An empty description SHALL show the name only, with no placeholder. A failed load SHALL show the page's existing error text and no entries. Acceptance: component tests over the expected list file and over a failing load.
- **R-12** Names and descriptions SHALL render as text only, never through `{@html}`, a markdown renderer or a sanitizer. Acceptance: the description `<img src=x onerror=alert(1)> **b**` creates no `img` or `strong` element, and the rendered text equals the input.

**Removals** (D-3; Peter's ruling: "Remove the editor")
- **R-13** Remove the following, along with their client calls and displays: `GET /api/projects` and `GET /api/projects/{slug}` (`libs/site-core/routes/projects.rs`); `frontend/src/routes/(public)/projects/[slug]/+page.svelte`; the admin editor `frontend/src/routes/admin/projects/+page.svelte`; `/api/admin/projects` (`libs/site-core/routes/admin/projects.rs`); the projects count in `libs/site-core/routes/admin/dashboard.rs`. The `projects` table and its rows SHALL stay, and no migration SHALL be added. Acceptance: no removed API path returns `application/json`, and `git diff --stat` lists nothing under `migrations/`.

**CI** (QA-5)
- **R-14** With no list file and no network, `cargo test --workspace` SHALL pass, and so SHALL `npm run check`, `npm run test:unit` and `npm run build` run from `frontend/`. The script's tests SHALL run inside one of those commands. Acceptance: all four pass offline on a fresh checkout, and reverting any single rule of R-2 to R-4 or R-7 makes a test fail.
- **R-15** This change SHALL NOT modify `.github/workflows/test.yml`, the `Dockerfile`, or any existing step of `deploy.yml` or its order. Its only workflow change is new steps in the `deploy` job above `Deploy`. Acceptance: `git diff` on those three files shows only added lines, all in the `deploy` job above `- name: Deploy`.

## Security
- **The GitHub token in CI.** The token is scoped to the job, read from the environment and never logged (R-5). The credential's authority is unchanged. A 401 or 403 fails the job (R-2). `FLY_API_TOKEN` stays in the `Deploy` step alone. Any new step uses only Actions the test job already runs, such as `actions/setup-node`.
- **Repo text flowing into the page and the AI prompt.** Only Peter's account writes it. The coordinator measured `mnemra` at one member, prmanahan, with no outside collaborators. The text is still treated as input: validation fails closed against a closed name pattern (R-4); links are composed, never taken from GitHub (R-7); rendering is text only (R-12); in the prompt, the text sits in the projects section after the security block (R-10). Residual: a description could carry instructions to the model. The only control against that is the model's instructions, which is true of all prompt content today.
- **A stale or fixture list ships.** Guarded by a fresh fetch on every deploy, deleting the file before fetching, and no fallback (R-2). The output is untracked and fixtures use `fixture-` names (R-8). The server refuses to start without a valid file (R-9). A manual `flyctl deploy` outside CI is not covered.
- **FL-4, secret scanning:** a repo-wide gap. Per Peter's "Separate repo fix" it is filed as its own task and does not block this change.

## Out of Scope
- Articles. Writing or applying descriptions (Ink drafts them; Peter applies them on GitHub).
- Dropping the `projects` table. Parked on #3554, to resurface when that spec is next revised.
- A post-deploy check of the list (the incoming post-deploy CSP check sits in that spot). Manual deploys. A cap on repo count or description length.
- The chat and fit transports (`ai/anthropic_stream.rs`, `ai/stop_reason.rs`), the Astro pages, the content bundle.

## Scenarios
- **Given** GitHub lists `mnemra`: `mnemra-core`, `mnemra.dev`, `.github` and `prmanahan`: `folio`, `sigil`, `factor` plus three forks, **when** `main` deploys, **then** /projects shows `mnemra` (`mnemra-core`, `mnemra.dev`) and then `prmanahan` (`factor`, `folio`, `sigil`), and the prompt lists the same five.
- **Given** the API returns 503, **when** the deploy job runs, **then** the script exits non-zero naming the 503, `Deploy` does not run, and the live release is unchanged.

## Data Model
**List file**, served at `/projects.json`. This is a public contract, and no field outside this table appears at any level.
| Field | Type | Constraints |
|---|---|---|
| `source` | string | `"github"` |
| `fetched_at` | string | RFC 3339, UTC |
| `groups` | array | exactly `mnemra`, then `prmanahan` |
| `groups[].owner` | string | `mnemra` or `prmanahan` |
| `groups[].repos` | array | at least one entry; sorted by name, case-insensitive |
| `repos[].name` | string | `^[A-Za-z0-9._-]{1,100}$` |
| `repos[].description` | string | GitHub's description, or `""` when GitHub's is null |
| `repos[].url` | string | `https://github.com/<owner>/<name>` |

## First real run
The first real run is the merge to `main`, because the merge deploys. It is attended and runs on live GitHub data, after Peter confirms the new descriptions are applied. It needs R-1 to R-13 and has no toggle.

Observe: the job log's included and excluded repos match the first scenario. /projects shows the five repos in two groups. `/projects.json` has `source` set to `github` and a `fetched_at` that matches the log. This confirms that `flyctl` uploads the untracked file and that the CDN serves no older copy. Record the `cf-cache-status` header. Asked about Peter's projects, the chat names only those repos, and never Puck Playbook. Mitigation: revert the merge commit.

## Decision levels
- **3, recommend and wait for approval:** a scope-bearing premise, your own included, raised the moment you hold it and never parked for retro; a new dependency; the first run on real data, with Bolt walking the runbook before it; any change to the floor, actor set, bar or known-gaps list, signed or not; a decision to decline a floor control; the 5-round ask. A decision that touches an actor, a floor, the bar or a known gap counts as that stop. In doubt, it touches.
- **4, act and tell at once:** reversible, but the decomposer may want to overturn it.
- **5, act and report at the routine update:** gate mechanics, reversible choices, anything where building proceeds either way.

| Decision this build will meet | Level | Answer, or the default the builder takes |
|---|---|---|
| The first real run | 3 | Peter confirms the descriptions are applied, and Bolt walks the runbook before the merge |
| Any new dependency | 3 | None expected (R-1) |
| The removals in R-13 | durable | Removed, by ruling. Listed by name at the gate |
| The list file's fields and the URL `/projects.json` | durable | As in **Data Model** |
| Repo ordering | 4 | A to Z, case-insensitive (D-6) |
| Local development needs the file | 4 | A `just` recipe runs the script. The token comes from the environment, never argv |
| Script location, and how its tests join `npm run test:unit` | 5 | Follow the house precedent `scripts/gen-invisible.mjs`; for example, widen the vitest `include` |
| Node setup in the deploy job, retries (at most 2, never a substitute list), log wording | 5 | Builder's call |

## Intent self-report
I read the JTBD as one list, fetched on every deploy and read by both the page and the AI. R-13 removes the detail pages, which the read-back did not mention. I took that from "Puck Playbook gone" and "just list".

## Dismissed review flags · Carried review findings (decomposer override)
None.
