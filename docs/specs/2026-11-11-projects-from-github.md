---
# Stamped by Puck at spec-exit; leave `draft` until then.
status: draft            # draft | locked
lock_tag: ""             # brief/projects-from-github/spec-locked, stamped at spec-exit
---

# Feature: projects page from GitHub

> **Spec for:** folio task #4726
> **Date:** 2026-11-11
> **Design ref:** Frame `docs/intent/projects-from-github-frame.md` and intake `docs/intent/projects-from-github.md`. Both are draft, and neither lock tag is cut yet. The plan pins the Frame by `brief/projects-from-github/frame-locked` once Puck cuts it.

## Purpose

The projects page and the site's AI show exactly the public repos of the GitHub owners `mnemra` and `prmanahan`, grouped by owner, each with its GitHub description. The list is fetched once per deploy and read by both from one file, so the page and the AI cannot disagree, and the old database projects (Puck Playbook among them) leave every public and AI path.

This spec applies the Frame's directions D-1 to D-8, each anchored there. It adopts the existing startup-refusal shape of `validate_static_dir` (`libs/site-core/static_files.rs`) for the list file.

Relation to the parked content bundle (`docs/specs/2026-09-14-content-bundle.md`): this change takes nothing that spec needs.
- It removes the public project JSON routes early, as that spec's admin-and-routes removal requirement would.
- It leaves the `projects` table for that spec's table-drop requirement.
- Its list file is one artifact with a locked field table, so another producer could emit it later.

The open tension with the folio-simplification Frame is the Frame's E-1. It stays with Peter.

## Requirements

**Fetch: in the deploy job, before `Deploy`** (Frame D-1, D-4, D-8)

- **R-1** The deploy job in `.github/workflows/deploy.yml` SHALL produce the list file on every run, before its `Deploy` step, from the GitHub REST API. It SHALL list the repositories of exactly two owners, `mnemra` and `prmanahan`, and SHALL follow pagination until the API reports no further page.
  - Acceptance: with a test double serving an owner's repos across two pages, the produced file contains the repos from both pages.
- **R-2** The fetch SHALL exit non-zero, write no list file, and so stop the job before `Deploy`, when any of these holds:
  - a request fails at the network level or returns a non-2xx status (401 and 403 included);
  - a response body does not parse as a repository array;
  - pagination cannot be completed;
  - an owner has zero repos after the exclusions in R-4;
  - any element fails R-5.
  - Acceptance: one seeded case per condition, each giving a non-zero exit and no file at the output path.
- **R-3** The fetch SHALL NOT substitute any other list when it fails. That rules out a fixture, a cached copy, a committed copy and an earlier deploy's list.
  - Acceptance: with the API base set to an unreachable address, the run exits non-zero even when a valid list file already sits at the output path and the committed fixtures are present in the tree.
- **R-4** Every repo returned for an owner SHALL appear in the list unless it matches one of exactly three exclusions:
  - `fork` is true;
  - `archived` is true;
  - its name equals `.github`, compared case-insensitively.

  No other exclusion, allowlist or per-repo setting SHALL exist.
  - Acceptance: a fixture repo whose name appears nowhere else in the repository is listed. A fork, an archived repo and a `.github` repo in the same fixture are not listed.
- **R-5** The fetch SHALL fail (R-2), and SHALL NOT skip the element, when a returned element meets any of these:
  - `private` is anything other than `false`;
  - its owner login differs from the owner requested, compared case-insensitively;
  - its name does not match `^[A-Za-z0-9._-]{1,100}$`;
  - its description is neither null nor a string.
  - Acceptance: one seeded element per condition gives a non-zero exit.
- **R-6** The fetch SHALL authenticate with the job's automatically issued `GITHUB_TOKEN`, received through an environment variable.
  - The workflow's `permissions` SHALL remain `contents: read`.
  - No personal access token, repository secret or environment secret SHALL be added.
  - The token SHALL NOT appear in a command-line argument, in the list file or in any log line.
  - `FLY_API_TOKEN` SHALL NOT be in the fetch step's environment.
  - Acceptance:
    - the `permissions` block and the secret references in `deploy.yml` are unchanged except for the fetch step's `GITHUB_TOKEN` env entry;
    - a run with a sentinel token value finds the sentinel in neither the output file nor the captured stdout or stderr.
- **R-7** The fetch SHALL write to the job log, for each owner:
  - the count of repos included and each included name;
  - each excluded name with its exclusion reason;
  - each included name whose description is empty.

  On failure it SHALL write the condition that failed.
  - Acceptance: for the shared fixture, the captured output contains each of those lines.

**The list file** (Frame D-5, D-6, D-7)

- **R-8** The list file SHALL have exactly the fields in **Data Model**.
  - Groups SHALL be ordered `mnemra` then `prmanahan`.
  - Repos within a group SHALL be ordered by name, ascending and case-insensitive.
  - `url` SHALL be composed as `https://github.com/<owner>/<name>` from the validated owner and name.
  - No other field from GitHub, such as `homepage` or `html_url`, SHALL be held.
  - Acceptance: for the shared fixture, the produced file equals a committed expected file byte for byte, apart from `fetched_at`.
- **R-9** The generated list file SHALL NOT be tracked in git, and tests SHALL use committed fixtures whose repo names all begin `fixture-`.
  - Acceptance: after running the fetch in a clean checkout, `git status --porcelain` prints nothing.
  - Acceptance: every repo name in every committed list fixture begins `fixture-`.

**Server** (Frame D-2)

- **R-10** At startup the server SHALL read the list file from the static directory. It SHALL refuse to start, exiting non-zero with a message naming the file's path, when the file is absent, does not parse, violates **Data Model**, or has an owner with zero repos.
  - Acceptance: one spawned-binary case per condition.
- **R-11** At startup the server SHALL log the list's `source`, its `fetched_at` and the repo count per owner.
  - Acceptance: the captured startup log for the shared fixture contains all three.
- **R-12** The projects section of the AI system prompt, built in `libs/site-core/ai/context.rs`, SHALL be built from the loaded list only.
  - It has one entry per repo, giving owner, name, `url`, and the description only when it is non-empty.
  - It keeps its existing position in the section order, after the security block.
  - It SHALL NOT read the `projects` table.
  - Acceptance: with the table holding a published row titled `Puck Playbook` and the shared fixture loaded, the prompt contains every fixture repo name and does not contain `Puck Playbook`.
- **R-13** The server SHALL make no request to GitHub at any point.
  - Acceptance: with networking disabled and a valid list file present, the server starts, `GET /projects.json` returns 200, and the prompt builds.

**Page** (Frame D-2, D-5, D-7; R-18 anchors V-Accessibility, the floor on a surface others use)

- **R-14** `/projects` SHALL load the list file from the same origin at runtime and render:
  - one group per owner, in file order, under a heading that names the owner;
  - one entry per repo, in file order, whose name links to its `url` with `target="_blank" rel="noopener noreferrer"`, followed by its description.
  - Acceptance: a component test over the shared fixture finds the groups, names, links and descriptions in file order.
- **R-15** The page SHALL render names and descriptions as text only. They SHALL NOT pass through `{@html}`, a markdown renderer or a sanitizer.
  - Acceptance: a description of `<img src=x onerror=alert(1)> **b**` creates no `img` or `strong` element in the list, and the entry's text equals the input exactly.
- **R-16** A repo whose description is empty SHALL show its linked name with no description text and no placeholder.
  - Acceptance: the entry's text content equals the repo name.
- **R-17** When the list file cannot be loaded or parsed, `/projects` SHALL show the page's existing error message and no entries.
  - Acceptance: a component test with a failing load finds the error element and zero entries.
- **R-18** The group headings SHALL be one level below the page's `h1`, and each link's accessible name SHALL be the repo name.
  - Acceptance: `npx playwright test e2e/accessibility.spec.ts` passes its `/projects` cases with the list file mocked.

**Removals** (Frame D-3)

- **R-19** The following SHALL be removed:
  - `GET /api/projects` and `GET /api/projects/{slug}` (`libs/site-core/routes/projects.rs`, merged in `libs/site-core/routes/mod.rs`);
  - the page `frontend/src/routes/(public)/projects/[slug]/+page.svelte`;
  - the public client calls for them in `frontend/src/lib/api.ts`.
  - Acceptance: neither path returns an `application/json` response.
  - Acceptance: `git grep -n "getProject"` prints no line under `frontend/src/lib/api.ts`.

**CI** (Frame QA-5; R-22 anchors P-TestableByDesign)

- **R-20** `cargo test --workspace`, and from `frontend/` `npm run check`, `npm run test:unit` and `npm run build`, SHALL pass with no generated list file present and no network.
  - Acceptance: all four commands pass on a fresh checkout with networking disabled.
- **R-21** This change SHALL NOT modify `.github/workflows/test.yml`, or any existing step of either job in `.github/workflows/deploy.yml`, or their order. Its only workflow change SHALL be new steps in the `deploy` job, placed before `Deploy`.
  - Acceptance: `git diff` on the two workflow files shows only added lines, all inside the `deploy` job and above `- name: Deploy`.
- **R-22** The exclusion, validation, ordering and failure behaviour of R-2 to R-5 and R-8 SHALL be exercised by tests that run in the existing CI gate against committed fixtures, with one seeded case per failure condition.
  - Acceptance: reverting any one exclusion, validation or ordering rule in the fetch code makes at least one test in that gate fail.

## Out of Scope

- Articles, and any article route or content.
- Writing or applying the GitHub descriptions. Ink drafts them and Peter applies them on GitHub.
- Any change to admin project editing, `/api/admin/projects` or the `projects` table beyond what the level-3 row below answers.
- A post-deploy check of the list. One is deliberately not added, because the in-flight post-deploy CSP check appends to the same place. R-2 and R-10 carry the fail-loud promise.
- Deploys not run by `deploy.yml`, such as a manual `flyctl deploy`. They ship whatever list file is in the local tree. The freshness promise covers the CI path only (Frame T-4).
- A cap on repo count or description length. This is Warden's item T-2. Adding one touches an actor, so it is a level-3 change.
- The Astro pages, the content bundle, and resolution of Frame E-1.
- The chat and fit transport paths (`ai/anthropic_stream.rs`, `ai/stop_reason.rs`).

## Scenarios

### Scenario: first deploy after the descriptions are applied
**Given** GitHub lists `mnemra`: `mnemra-core`, `mnemra.dev`, `.github`, and `prmanahan`: `folio`, `sigil`, `factor` plus three forks
**When** `main` deploys
**Then** /projects shows `mnemra` with `mnemra-core` and `mnemra.dev`, then `prmanahan` with `factor`, `folio` and `sigil`, and the AI prompt lists the same five.

### Scenario: GitHub unreachable
**Given** the API returns 503
**When** the deploy job runs
**Then** the fetch exits non-zero naming the 503, `Deploy` does not run, and the live release is unchanged.

### Scenario: a new public repo
**Given** Peter creates public repo `example` under `prmanahan`
**When** the next deploy runs, with no folio edit
**Then** `example` appears under `prmanahan` on /projects and in the prompt.

### Scenario: a non-public element is returned
**Given** a response element has `private: true`
**When** the fetch validates it
**Then** the fetch exits non-zero and writes no file.

### Scenario: markup in a description
**Given** a description `<script>x</script>`
**When** /projects renders it
**Then** the text `<script>x</script>` is shown and no `script` element exists in the list.

### Scenario: empty description
**Given** a repo with a null description
**When** the list is built
**Then** its `description` is `""`, the page shows its linked name only, the prompt entry has no description line, and the job log names it.

## Data Model

**List file.** Served at `/projects.json`, a public contract. Read by the page and the server.

| Field | Type | Constraints |
|---|---|---|
| `source` | string | `"github"` in a fetched file |
| `fetched_at` | string | RFC 3339, UTC, the time of the fetch |
| `groups` | array | exactly two entries: `mnemra`, then `prmanahan` |
| `groups[].owner` | string | `mnemra` or `prmanahan` |
| `groups[].repos` | array | at least one entry, ordered by name, case-insensitive |
| `repos[].name` | string | matches `^[A-Za-z0-9._-]{1,100}$` |
| `repos[].description` | string | GitHub's description; `""` when GitHub's is null |
| `repos[].url` | string | exactly `https://github.com/<owner>/<name>` |

No other field at any level.

## API Contract

### `GET /api/projects` and `GET /api/projects/{slug}`: removed
These return no `application/json` response after this change (R-19).

### `GET /projects.json`: a static file
This is the list file, served by the existing static file service. No new route is added.

## UI Behavior

- /projects shows one section per owner with the owner's name as its heading. Each repo appears as its linked name and its description.
- The repo link opens GitHub in a new tab.
- While the list loads, the page shows nothing new. On a load failure it shows the existing error text.
- The existing projects grid stays responsive with no horizontal overflow (`e2e/responsive.spec.ts`).

## Constraints

- Integrate with the existing static file service and the existing `validate_static_dir` startup pattern.
- No compile-time embedding (`decisions/DEFAULTS.md` G-0002).
- Adding a dependency of any kind is level 3.
- Leave the chat and fit paths' asymmetry intact (CLAUDE.md, spec #572).

## First real run

**The landing of this change on `main` is the first real run,** because the merge deploys.
- **Runs on:** live GitHub data for both owners, after Peter confirms he has applied the new descriptions.
- **Attended:** yes. Peter, or Puck on his behalf, watches the deploy job.
- **Needs:** R-1 to R-19. There is no toggle.

**Observed:**
1. The job log lists `mnemra`: `mnemra-core`, `mnemra.dev`, and `prmanahan`: `factor`, `folio`, `sigil`. It excludes `.github` and the three forks, with reasons.
2. https://peter.manahan.io/projects shows those five in two groups.
3. `/projects.json` reports `source` `github`, and its `fetched_at` matches the job log. This confirms that `flyctl` uploaded the untracked file and that the CDN served no older copy, both INFERRED until this run. Its `cf-cache-status` header is recorded.
4. The chat, asked what Peter has built, names repos only from those five and never Puck Playbook.

**Mitigation path:** revert the merge commit, which redeploys.

## Decision levels

Every decision this build meets has a level. "Wait until told" and "ask what to do" are not levels.

- **3, recommend and wait for approval:** a scope-bearing premise, your own included, raised the moment you hold it and never parked for retro; a new dependency; the first run on real data, with Bolt walking the runbook before it; any change to the floor, actor set, bar or known-gaps list, signed or not; a decision to decline a floor control; the 5-round ask. A decision that touches an actor, a floor, the bar or a known gap counts as that stop. In doubt, it touches.
- **4, act and tell at once:** reversible, but the decomposer may want to overturn it.
- **5, act and report at the routine update** (the grant's reporting point or the landing's report): gate mechanics, reversible choices, anything where building proceeds either way.

A choice no row answers that is durable or rests on context the builder lacks is raised before acting. Levels never close a finding or replace a gate signature.

| Decision this build will meet | Level | Answer, or the default the builder takes |
|---|---|---|
| **The `projects` table and admin project editing.** Options: (1) keep the admin editor, `/api/admin/projects` and the table unchanged, so admin project edits become dead writes nothing reads (the read-back's literal reading); (2) remove the admin projects editor (`frontend/src/routes/admin/projects/+page.svelte`), `/api/admin/projects` (`libs/site-core/routes/admin/projects.rs`) and the dashboard's projects count, keeping the table and its rows with no migration; (3) as (2), plus a migration that drops the table | 3 | **Recommend (2).** An editor that silently changes nothing is a silent-failure class, so it is removed on first sighting (P-GuaranteeByMechanism). Without a migration it stays reversible. The bundle spec's table-drop requirement already names `projects`, so the drop is **parked** on #3554 and resurfaces when that spec is next revised. (1) is cheapest and matches the read-back literally. |
| **Fetch implementation language.** Options: (a) Rust in this workspace, sharing one validator with the R-10 startup check; the deploy job installs the pinned toolchain, and HTTP goes through `rig_core::http_client`, already used by `libs/site-core/ai/anthropic_stream.rs` (its fit for a paginated GET is unverified), or else `reqwest` as a new direct dependency; (b) `gh api` and `jq` in the workflow, where the logic sits in YAML and cannot meet R-22 without a second-language script; (c) a Node script under `frontend/`, tested with vitest | 3 | **Recommend (a)** (P-RustDefault: "Use Rust for anything the workspace builds and runs"; P-StackDiscipline). Its costs: build time in the deploy job, and the toolchain actions running in the `production` job (ACTOR-6). (b) and (c) are departures that only Peter grants. |
| Any new dependency | 3 | Recommendation and wait |
| The first real run | 3 | Peter confirms the descriptions are applied. Bolt walks the runbook before the merge. |
| Removing the public project routes and detail page (R-19) | durable | Removed. Listed by name at the gate. |
| The list file's fields and its URL `/projects.json` (R-8, Data Model) | durable | As specified. Listed by name at the gate. |
| Repo ordering (R-8) | 4 | Name A to Z, case-insensitive (Frame D-6) |
| Where the generated file sits in the tree | 4 | A gitignored path that the frontend build copies to the build root, for example under `frontend/static/` |
| Local development needs the file to start | 4 | A `just` recipe runs the same fetch locally (G-0006). A local token comes from the environment, never argv. |
| Retries on a failed request | 5 | At most 2, then fail. Never a substitute list. |
| Log wording, heading markup, e2e mocks of the list | 5 | Builder's call |

## Intent self-report

Non-normative. The Requirements govern.

- **JTBD reading.** I read the JTBD as one list of the two owners' public repos, fetched each deploy and read by both page and AI, with the database projects gone from every public and AI path.
- **Strain, R-19.** It removes the detail pages, which the read-back did not mention. I take this from "Puck Playbook gone" and "just list".
- **Strain, level-3 table and admin row.** Option (2) or (3) strains the non-goal "Admin project editing … stay as they are for this change".

## Dismissed review flags

None.

## Carried review findings (decomposer override)

None.
