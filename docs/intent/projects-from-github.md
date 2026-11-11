---
spec_type: code
frame_relevant: true
---

# Intake: projects page from GitHub

**Date:** 2026-11-11
**Status:** draft
**Task:** #4726
**Consumer:** mixed: site visitors (recruiters especially) read the page; the site's AI reads the same list; build agents read this doc
**Kind:** other: a build-time step in the deploy pipeline, plus the existing projects page and AI context reading its output. No new server runtime surface.
**Path:** Lightweight (Stage 1 and Stage 2 collapsed). The read-back ran questions 1 to 3 and 8, as that path allows.

Labels: **PETER** = Peter's words, verbatim, 2026-11-11. **MEASURED** = checked in this repository at `23088bb`. **INHERITED** = taken from the dispatch and not re-checked. **INFERRED** = the author's reasoning.

## Read-back

Peter's words, in order (PETER):

- "we are going to have to update the projects and articles on the site."
- "the projects page should likely just list the ones from mnemra.org and prmanahan github, with a short description of each. They should be grouped by org."
- On whether descriptions come from GitHub or folio: "rewrite them on github, draft the descriptions for me"
- "the articles can wait a bit, lets get the projects clean first"
- "build it at build time, drop puck playbook"

Puck's read-back, confirmed by Peter with "yes, those are right, go ahead":

1. **Job.** Show visitors, recruiters especially, what Peter builds. List every public repo in the `mnemra` GitHub org and the `prmanahan` account, grouped by owner, each with its GitHub description, with Puck Playbook gone. The site's AI gets its project knowledge from the same list, so the page and the AI can't disagree.
2. **Kind.** A build-time step that fetches the list from GitHub, plus the existing projects page showing it. No new server runtime surface.
3. **Earliest real run.** A production deploy after Peter has applied the new descriptions on GitHub. /projects shows the five current repos in two groups. Admin project editing and articles stay as they are for this change.
8. **Promise words.**
   - **"clean"** means exactly the public repos of those two owners, excluding forks, `.github` and archived repos, and nothing else.
   - **"build time"** means fetched during the deploy build, not per visitor, so a new repo or changed description appears on the next deploy.
   - If GitHub can't be reached during the build, the deploy fails loudly rather than shipping an empty or stale list.

"mnemra.org" in Peter's words is the GitHub org `mnemra`; its site is mnemra.dev (INHERITED).

## JTBD

A visitor needs to see what Peter builds: the public repositories under `mnemra` and `prmanahan`, grouped by owner, each with a short description. When the visitor asks the site's AI about Peter's projects, it should give the same answer the page shows.

## Non-goals

- **Articles.** "the articles can wait a bit" (PETER). This change makes no article edits.
- **Writing or applying the descriptions.** Ink drafts them, and Peter applies them on GitHub himself (INHERITED). That work sits outside this spec.
- **Per-visitor or runtime calls to GitHub.** "build time" (PETER, promise word).
- **Any repository beyond the two owners, and any curation within them.** "nothing else" (PETER, promise word).
- **Admin project editing for the first run.** "Admin project editing and articles stay as they are for this change" (PETER, read-back 3). ⚠ The dispatch reads this as what can stay off for the first run, not as a ruling to keep admin project editing permanently. The spec puts the question to Peter at level 3, so the intake does not settle it.

## Success criteria

1. After the first production deploy, /projects shows exactly five repos in two groups: `mnemra` with `mnemra-core` and `mnemra.dev`, and `prmanahan` with `factor`, `folio` and `sigil`. (The repo set is INHERITED from the dispatch's 2026-11-11 `gh repo list` measurement. The group membership follows from the exclusion rule.)
2. Neither /projects nor the AI's system prompt contains "Puck Playbook".
3. The set of repos and descriptions in the AI's system prompt equals the set /projects renders from the same deploy.
4. A public repo created under either owner after this change appears on /projects after the next deploy, with no edit to folio.
5. When GitHub can't be reached, or returns an unusable response during the deploy build, the deploy job fails and the live site keeps its previous release.

## Hard constraints

- **Fetched at build time, never per visitor** (PETER). Production has no runtime dependency on GitHub.
- **Fail loudly, never empty or stale** (PETER, promise word). This is ruled and is not reopened.
- **No new server runtime surface** (PETER, read-back 2).
- **No compile-time embedding.** The list is read at runtime from a file (`decisions/DEFAULTS.md` G-0002; CLAUDE.md guardrail).
- **Deploys go through CI.** Merging to `main` deploys via `.github/workflows/deploy.yml` (CLAUDE.md guardrail, MEASURED).
- **Two other branches will edit `.github/workflows/deploy.yml` and `test.yml`:** a CSP check step and a post-deploy CSP check (INHERITED). This change must not collide with them.
- **The two AI paths stay asymmetric** (CLAUDE.md, spec #572). This change touches only the projects section of the shared context, not the chat or fit transport.
- **This change must not foreclose the parked content bundle** (`docs/specs/2026-09-14-content-bundle.md`, #3554) (INHERITED).

## Evidence

- Peter's words and the confirmed read-back above (PETER).
- MEASURED, current state: `frontend/src/routes/(public)/projects/+page.svelte` loads `/api/projects`. That route (`libs/site-core/routes/projects.rs`) reads the `projects` table through `libs/site-core/models/project.rs`, and admin manages the table (`libs/site-core/routes/admin/projects.rs`, `frontend/src/routes/admin/projects/+page.svelte`). The AI context reads the same table through `project::list_all` (`libs/site-core/ai/context.rs:41`).
- MEASURED: `git grep -i playbook` over the tracked files returns nothing. The Puck Playbook entry therefore lives only in the production database (INFERRED).
- INHERITED, measured 2026-11-11 with `gh repo list <owner> --visibility public`: `mnemra` has `mnemra-core`, `mnemra.dev` and `.github`. `prmanahan` has `folio`, `sigil` and `factor`, plus forks of `mnemra-core`, `mnemra.dev` and `.github`.

## Risk profile

May touch a trust boundary: **yes**. Text from an outside system (GitHub repo names and descriptions) flows into a public page and into the AI's system prompt, and the deploy job uses a GitHub credential. This is a required Stage 2 constraint, resolved in the Frame.

## Security implications (draft scope)

Not run. Warden's discovery-stage read did not run on this Lightweight path. The Frame drafts the threat-model scope from the Frame alone, and says so.

## Consultations

None.

## Dismissed review flags

None.
