---
spec_type: code
frame_relevant: true
---

# Intake: projects page from GitHub

**Date:** 2026-11-11 · **Status:** draft · **Task:** #4726 · **Path:** Lightweight (read-back questions 1 to 3 and 8)
**Consumer:** mixed. Visitors, recruiters especially, read the page, and the site's AI reads the same list.
**Kind:** a build-time step in the deploy pipeline, plus the existing projects page and AI context reading its output.

## Read-back

Peter's words, verbatim, 2026-11-11:
- "we are going to have to update the projects and articles on the site."
- "the projects page should likely just list the ones from mnemra.org and prmanahan github, with a short description of each. They should be grouped by org."
- "rewrite them on github, draft the descriptions for me"
- "the articles can wait a bit, lets get the projects clean first"
- "build it at build time, drop puck playbook"

Puck read the items below back to him, and he confirmed them with "yes, those are right, go ahead":
1. **Job:** show visitors, recruiters especially, what Peter builds. List every public repo in the `mnemra` GitHub org and the `prmanahan` account, grouped by owner, each with its GitHub description, with Puck Playbook gone. The site's AI gets its project knowledge from the same list, so the page and the AI can't disagree.
2. **Kind:** a build-time step that fetches the list from GitHub, plus the existing projects page showing it. No new server runtime surface.
3. **Earliest real run:** a production deploy after Peter has applied the new descriptions on GitHub. /projects shows the five current repos in two groups. Admin project editing and articles stay as they are for this change.
8. **Promise words:**
   - "clean" means exactly the public repos of those two owners, excluding forks, `.github` and archived repos, and nothing else.
   - "build time" means fetched during the deploy build, not per visitor, so a new repo or changed description appears on the next deploy.
   - If GitHub can't be reached during the build, the deploy fails loudly rather than shipping an empty or stale list.

"mnemra.org" means the GitHub org `mnemra`; its website is mnemra.dev.

## JTBD

A visitor needs to see what Peter builds: the public repos under `mnemra` and `prmanahan`, grouped by owner, each with a short description. The site's AI should answer from the same list.

## Non-goals

- Articles ("the articles can wait a bit").
- Writing or applying the descriptions: Ink drafts them and Peter applies them on GitHub.
- Calling GitHub per visitor or at runtime.
- Any repo outside the two owners, and any curation within them ("nothing else").

Read-back 3 said admin project editing stays as it is. Peter has since ruled "Remove the editor" (2026-11-11), so that non-goal no longer holds.

## Success criteria

1. After the first deploy, /projects shows `mnemra` (`mnemra-core`, `mnemra.dev`) and `prmanahan` (`factor`, `folio`, `sigil`).
2. Neither /projects nor the AI prompt contains "Puck Playbook".
3. The AI prompt and /projects show the same repos and descriptions.
4. A new public repo under either owner appears after the next deploy with no edit to folio.
5. If GitHub can't be reached, or returns something unusable, the deploy job fails and the live release stays as it was.

## Hard constraints

- The list is fetched at build time, and a failed fetch fails the deploy loudly. Both are ruled.
- No new server runtime surface.
- No compile-time embedding (`decisions/DEFAULTS.md` G-0002).
- Deploys go through `.github/workflows/deploy.yml`.
- The incoming security branch also edits that workflow, `test.yml` and the `Dockerfile`. This change must not collide with it.
- The chat and fit AI paths stay asymmetric (CLAUDE.md, spec #572).
- Nothing here may foreclose the parked content bundle (`docs/specs/2026-09-14-content-bundle.md`, #3554).

## Evidence

- Measured at `23088bb`:
  - `frontend/src/routes/(public)/projects/+page.svelte` loads `/api/projects`.
  - That endpoint (`libs/site-core/routes/projects.rs`) and the AI context (`project::list_all`, `libs/site-core/ai/context.rs:41`) both read the `projects` table, which admin manages.
  - `git grep -i playbook` finds nothing in the tracked files, so the Puck Playbook entry lives only in the production database.
- From the coordinator's `gh` measurements, 2026-11-11, which I did not re-run:
  - `mnemra` has `mnemra-core`, `mnemra.dev` and `.github`.
  - `prmanahan` has `folio`, `sigil` and `factor`, plus forks of the three `mnemra` repos.
  - The `mnemra` org has exactly one member, prmanahan, and no outside collaborators.

## Risk profile

The change crosses a trust boundary in two ways:
- GitHub text flows into a public page and into the AI prompt.
- The CI job uses a GitHub token.

These are resolved in the Frame and the spec's Security section.

## Consultations · Dismissed review flags

None.
