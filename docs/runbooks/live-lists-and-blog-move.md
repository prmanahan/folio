# Live lists and blog move: rollback and break-glass runbook

Source: `docs/specs/2026-11-18-live-lists-and-blog-move.md`, blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`. The break-glass runbook and the merge-2 rollback apply only once merge 2 has deployed.

## First run, merge 1: unpublish the article

**Unpublished at first run** (Peter, 2026-11-20: "Unpublish at first run"; review finding ACTOR-6). After merge 1 nothing in the app can unpublish an article, and `libs/site-core/ai/context.rs` still puts every `articles` row with `published = 1`, full text included, into the chat and fit prompts until merge 2's T11 (R-20) moves them to mnemra.dev's RSS feed. This step sets "The Workflow Didn't Change" unpublished in the production DB so the prompts stop quoting it.

**Draft, not run.** No step here has been executed against production. The first execution is Peter's attended stop, and the operator records the actual output of each command in the T6 report.

**Blast radius.** One `articles` row, one column. The data lives on the `site_data` volume (`/app/data/site.db`), so the change outlives any deploy or rollback. Nothing else in the DB is touched.

**When.** After the merge-1 deploy has finished and its post-deploy checks (T6's list) are seen, so a deploy failure is not tangled with a data change. Before declaring the first run closed.

**Reaching the DB.** The runbook had no DB access before this step. The runtime image (`debian:bookworm-slim`, `Dockerfile`) carries no `sqlite3` CLI, and the app has no subcommand that runs SQL. Unverified until Peter's run: `fly ssh console` logs in as root, and the machine can reach the apt mirrors.
1. `fly status -a folio-prmanahan`. **Stop unless exactly one machine is listed**: the volume is per machine, and a second machine holds a second DB.
2. `fly ssh console -a folio-prmanahan`. A stopped machine is started by this (`auto_stop_machines`).
3. In that shell: `command -v sqlite3 || (apt-get update && apt-get install -y sqlite3)`. The install goes to the machine's root filesystem, not the volume, and is gone at the next restart or deploy. That is intended: no image change, nothing to remember to remove. If the install fails, stop and report; do not copy the DB off the volume or edit it by another route.

**Steps** (all inside the ssh shell).
1. Identify the row; do not guess an id:
   ```
   sqlite3 -header -column /app/data/site.db "SELECT id, slug, title, published FROM articles WHERE title LIKE '%Workflow Didn%t Change%';"
   ```
   The `%` between `Didn` and `t` matches a straight or a curly apostrophe. **Expect exactly one row, with `published` = 1. Stop if there are zero rows, two or more rows, or `published` is already 0.** Zero or many means the title or the DB is not what this step assumes; already 0 means it was done, so go to step 4. Record the row: its `id` is `<ID>` below, and its `published` value is the reversal's target.
2. Optional positive control, to make step 4's prompt check meaningful: ask the chat on https://peter.manahan.io to quote the opening of "The Workflow Didn't Change", and record whether it can. The positive control may not succeed on any one ask, since the model can decline; a miss here weakens the prompt check in step 4 to corroboration.
3. Update, scoped to the id from step 1:
   ```
   sqlite3 /app/data/site.db "UPDATE articles SET published = 0 WHERE id = <ID>; SELECT changes();"
   ```
   Expect `1`. Any other value: stop.
4. Verify the row:
   ```
   sqlite3 -header -column /app/data/site.db "SELECT id, slug, title, published FROM articles WHERE id = <ID>;"
   ```
   Expect the same row with `published` = 0. This read is the decisive check: both prompts are built from `articles` on each request (`build_system_prompt` and `build_fit_prompt` take the DB connection; no cache sits in `ai/context.rs`), and filter on `published`, so a row at 0 is not in the next prompt.
5. Verify the prompt from outside: ask the chat the step-2 question again, and run a job-fit analysis with a pasted description that mentions the article's topic. Expect no quotation of the article's text or title. An absence here proves little alone (the model may simply not quote it); it counts only alongside step 4, or alongside a step-2 hit that no longer reproduces.
6. `exit`. The ssh session ends; nothing else to clean up.

**Reversal.** In the same kind of shell (repeat "Reaching the DB" steps 1 to 3):
```
sqlite3 /app/data/site.db "UPDATE articles SET published = 1 WHERE id = <ID>; SELECT changes();"
```
Expect `1`, then re-run step 4's SELECT and expect `published` = 1. This puts the article back in the prompts. Do not reverse it for a rollback unless that is the intent.

**Relative to the rollback blocks below.** The unpublish is data on the volume, so a rollback (merge 1 or merge 2) does not undo it. After a merge-1 rollback the old release serves `/articles` again, and it should no longer list or serve this one (expected from the `published` flag; not checked). Reverse it only if Peter wants the old page back, and know that doing so re-exposes the text to the prompts.

## Rollback, merge 1

**Rollback, merge 1.** `fly.toml` still runs `./folio check-projects` after merge 1.
- **Primary:** revert through `main`, which deploys and fetches a fresh list in CI. It needs GitHub up.
- **Fast path:** `fly deploy --image <recorded image> --skip-release-command -a folio-prmanahan`. The recorded image's baked `projects.json` is past its 30-minute bound, so without that flag the release command refuses the deploy.
- **After either,** the old release serves its own `/articles` pages again.

## Rollback, merge 2

**Rollback, merge 2.**
- **Primary:** redeploy merge 1's recorded image with `fly deploy --image <recorded image> -a folio-prmanahan`, from a post-merge-2 checkout. A pre-merge-2 checkout's `fly.toml` still runs `./folio check-projects`, and the image's baked `projects.json` is past its 30-minute bound, so the release command refuses the deploy.
- **Slow path:** revert merge 2 through `main`, which deploys. It needs GitHub up, because the reverted code fetches the list in CI.
- **After either,** the old release ignores the `live_lists` table and serves its own baked list, and its prompt reads the `articles` table. `/api/lists/freshness` answers the JSON 404, so the daily check is red every day; that is expected.
- **Rolling back for bad content:** set `LIVE_LISTS_DISABLED` for that list before rolling forward. Otherwise the poisoned row loads and is served from the first request.

## Break-glass runbook

**Break-glass runbook** (final wording; steps locked at Frame D-10):
1. `fly secrets set LIVE_LISTS_DISABLED=<projects|articles|projects,articles> -a folio-prmanahan`. If the machine does not restart on its own, restart it. At start the server deletes that list's stored row.
2. **Stop unless both hold:**
   - The start log line names the list with outcome `deleted` or `absent`. If it says `failed`, do not go on to step 5: the bad row is still stored.
   - `https://peter.manahan.io/api/lists/freshness` reports that list as `null`. For projects, `/api/lists/projects` also answers 503.

   If the list is not named, check the secret's name.
3. While the switch is set, the daily check goes red, and so does the post-deploy check on every deploy. That is the switch working. A misspelled value disables both lists and logs at `error`.
4. Correct the text at its source: the GitHub description, or the mnemra.dev post.
5. `fly secrets unset LIVE_LISTS_DISABLED -a folio-prmanahan`, restarting if needed. Confirm the list's time in the freshness read is later than the unset.
