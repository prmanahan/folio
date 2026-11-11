---
modulation: brownfield-extension
---

# Frame: projects page from GitHub

**Date:** 2026-11-11 · **Status:** draft · **Intake:** `docs/intent/projects-from-github.md` · **Task:** #4726

**Process.** Peter ruled "Trim, then light review" (2026-11-11). Under that ruling this change skips three steps: the threat-modeling pass, Wyrd's dry decompose and Charon's canon check. That is why this Frame carries no data-flow diagram, actor set or bar. Its security content is the spec's `## Security` section.

**Interim.** This change departs from the simplification Frame's L-4, which makes job the source of truth for all content (`docs/intent/folio-simplification-frame.md`). The departure is interim. Amending L-4 stays parked on #3554 (the coordinator's level-4 call, 2026-11-11).

## Quality-attribute scenarios

| ID | Stimulus | Environment | Response | Measure |
|---|---|---|---|---|
| QA-1 | A repo or its description changes on GitHub | the next deploy | the list is fetched fresh | the release never serves a list from an earlier run |
| QA-2 | GitHub is unreachable or its response is unusable | the deploy job | the job stops before `Deploy` | the job is red and the live release is unchanged |
| QA-3 | A visitor reads /projects and asks the AI | production | both read one file | the page and the prompt show the same set |
| QA-4 | A description carries markup | production | it is shown as text | no element is built from the description, and every link points to `https://github.com/<owner>/<name>` |
| QA-5 | A pull request runs CI | no network, no list file | tests run on committed fixtures | the existing test jobs pass, unchanged |

## Locked directions

| ID | Direction | Anchor | Rejected, and why |
|---|---|---|---|
| D-1 | The fetch runs as a new step in the CI `deploy` job, above `Deploy`. Its output goes into the build context that `flyctl deploy` uploads. | P-GuaranteeByMechanism: a fetch inside a Docker `RUN` could reuse the builder's layer cache and ship a stale list with no signal. The layer-cache behaviour is inferred and gets checked at the first real run. | **The Docker build:** a stale list could ship from the layer cache, and the remote builder has no token. |
| D-2 | One file, `/projects.json`, is placed in the frontend build output. The page loads it at runtime, and the server reads it at startup to build the AI context. | V-Simplicity: no new route and no Dockerfile change. G-0002: read at runtime, not embedded. Two readers of one file means QA-3 holds by construction. | **`/api/projects` serving a server-side copy:** changes the API contract and the Dockerfile. **A Vite build-time import:** the frontend build would need the file, so both test jobs would have to change. |
| D-3 | Remove `/api/projects`, `/api/projects/{slug}` and the detail page. Remove the admin projects editor, `/api/admin/projects` and the dashboard's projects count (Peter: "Remove the editor"). The `projects` table stays, with no migration. Dropping it is parked on #3554. | JTBD "Puck Playbook gone". P-GuaranteeByMechanism: an editor whose edits reach nothing is a silent failure. V-Reversibility: no migration is needed. | **Keeping the editor:** its edits would be dead writes. **Dropping the table now:** a migration the parked bundle spec already plans. |
| D-4 | Opt-out: every returned repo appears except forks, archived repos and `.github`. Validation fails closed. | Peter's "nothing else". P-SecurityLayered: "a control that errors denies rather than passes through". | **An allowlist:** a new repo would need an edit. **Skipping bad entries:** the control would fail open. |
| D-5 | Text-only rendering. Links are composed from owner and name, and no other GitHub field is held. | P-SecurityLayered: enumerate what is permitted. V-DataMinimization. | **The detail page's `{@html DOMPurify.sanitize(marked.parse(…))}`.** **GitHub's `homepage` or `html_url`.** |
| D-6 | Groups in the order `mnemra`, `prmanahan`. Repos sorted A to Z, ignoring case. | V-Simplicity: needs only the name, and the order is stable across deploys. | **Most recently pushed:** reorders on every push and needs a timestamp. |
| D-7 | An empty description shows the name only, and the deploy log names the repo. | V-Honesty: no invented text. V-Observability. | **A placeholder:** invented text. **Failing the deploy:** a missing description is not a broken fetch. |
| D-8 | The fetch authenticates with the job's built-in `GITHUB_TOKEN`. `permissions: contents: read` is unchanged, and `FLY_API_TOKEN` stays scoped to the `Deploy` step. | P-LeastAuthority: "scope a credential to the operations its role performs". | **A PAT:** a standing, account-wide credential. **No token:** shared runner addresses risk rate-limit false reds. |
| D-9 | The fetch is a plain Node script using the built-in `fetch`, with no new dependency, tested with `node:test` or vitest. Peter ruled "Node script". | This is a P-RustDefault departure that Peter granted. The house precedent is `scripts/gen-invisible.mjs`. | **Rust**, which canon recommended: a toolchain build in the deploy job. **`gh api` and `jq` in YAML:** untested logic. |

## Escalations, resolved 2026-11-11

- **Admin and table:** "Remove the editor" (D-3).
- **Language:** "Node script" (D-9).
- **FL-4, secret scanning:** "Separate repo fix". This is a repo-wide gap, filed as its own task, and it does not block this change.
- **Other `mnemra` writers:** the coordinator measured exactly one member, prmanahan, and no outside collaborators, so no other writer exists.
- **L-4 tension:** recorded as interim, see the top of this Frame.

## Intent self-report

I read the JTBD as one list of the two owners' public repos, fetched on every deploy and read the same way by the page and the AI. D-3 removes the detail pages, which the read-back did not mention. I took that from "Puck Playbook gone" and "just list".

## Carried review findings (decomposer override)

None.
