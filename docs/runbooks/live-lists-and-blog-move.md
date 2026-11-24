# Live lists and blog move: rollback and break-glass runbook

Source: `docs/specs/2026-11-18-live-lists-and-blog-move.md`, blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`. The break-glass runbook and the merge-2 rollback apply only once merge 2 has deployed.

## First run, merge 1: unpublish the article

**Route choice: ruled B (2026-11-20).** At merge 1's go Peter chose Route B ("I'm remote and I won't be able to access it easily so it's going to have to be option B: have Bolt do it"). Bolt ran it after the deploy, and article id 2 was unpublished. Route A (unpublish in the admin editor before the deploy) stays documented below as the primary for any future use: it needs no root shell, no package install and no second writer on the WAL database.

**Unpublished at first run** (Peter, 2026-11-20: "Unpublish at first run"; review finding ACTOR-6). After merge 1 nothing in the app can unpublish an article, and `libs/site-core/ai/context.rs` still puts every `articles` row with `published = 1`, full text included, into the chat and fit prompts until merge 2's T11 (R-20) moves them to mnemra.dev's RSS feed. This step sets "The Workflow Didn't Change" unpublished in the production DB so the prompts stop quoting it.

**Draft, not run, except Route B.** Route B ran on 2026-11-20 (see the route choice above). No other step here has been executed against production. The first execution of each is Peter's attended stop, and the operator records the actual output of each command in the T6 report.

**Output hygiene (both routes).** Record command output only. Do not run `env` or `printenv`, and do not paste terminal scrollback, the admin session cookie or a HAR.

**Blast radius.** One `articles` row. Route A rewrites the whole row: the editor's Save is `PUT /api/admin/articles/{id}` → `article::update` (main `d180810`, `libs/site-core/models/article.rs:136`), which writes title, slug, summary, content, tags, published_at and published from the form, and re-serialises tags, so stored tags that are not valid JSON come back as `[]`. Route B writes one column (`published`). The data lives on the `site_data` volume (`/app/data/site.db`), so the change outlives any deploy or rollback. Nothing else in the DB is touched. Route B adds a root session and a package install on the one production machine; Route A adds neither.

**When.**
- Route A: before the merge-1 deploy. The release serving production today (main `d180810`, deploy run green 2026-11-19) still has `/admin/articles` with a Published checkbox (`frontend/src/routes/admin/articles/+page.svelte`), backed by `PUT /api/admin/articles/{id}` and `article::update` (`libs/site-core/routes/admin/articles.rs`). Merge 1 (PR #80) deletes both. The data change therefore precedes the deploy, and a deploy failure is tangled with it: if the deploy is rolled back, the article stays unpublished (it is data on the volume) until it is re-published through the editor.
- Route B: after the merge-1 deploy has finished and its post-deploy checks (T6's list) are seen, so a deploy failure is not tangled with a data change. This is the only route once the editor is gone.
- Either way, before declaring the first run closed.

### Route A: admin editor, before the merge-1 deploy

Pre-condition: the production release is still the pre-merge-1 one. If `/admin/articles` is gone, use Route B.

- **A1. Record before.** Log in at `https://peter.manahan.io/admin`, open Articles, and record for every article its badge, and for "The Workflow Didn't Change" its title, slug, published date, its tags value as the editor shows it, and the character counts of its summary and content. The list's badges are the backup of the `published` column. Note the newest volume snapshot ID (`fly volumes snapshots list <volume id> -a folio-prmanahan`), as merge 2's pre-checks do. **Stop** unless exactly one article matches the title and its badge reads "published".
- **A2. Positive control (optional).** Ask the chat on https://peter.manahan.io to quote the opening of the article, and record whether it can. A miss weakens A5 to corroboration, since the model can decline.
- **A3. Unpublish.** Open the article, untick Published, Save.
- **A4. Verify the row.** Reload the article and the list. Expect the badge to read "draft", with title, slug, published date, tags value, and the summary and content character counts all matching A1. On any difference, stop and report: the reversal re-ticks Published but does not restore lost tags or text. A4 cannot see a tags loss when the stored tags were invalid JSON: the editor normalises them to `[]` on both sides. The A1 volume snapshot is the only record of them, and a restore from it is the recovery.
- **A5. Deploy, then check from outside.** A1 to A4 must all be finished before the merge is clicked. Merge PR #80 (CI deploys on push to main), run the post-deploy checks. Then ask the chat the A2 question again, and run a job-fit analysis with a pasted description that mentions the article's topic. Expect no quotation of the article's text or title. An absence proves little alone; it counts alongside A4, or alongside an A2 hit that no longer reproduces. Prompts are built per request from `articles` filtered on `published` (`build_system_prompt` and `build_fit_prompt` take the DB connection; no cache sits in `ai/context.rs`), so A4 is the decisive check.

**Route A reversal.** Before the deploy, or after a merge-1 rollback (the old release has the editor again): open the article, re-tick Published, Save, reload and expect the badge "published". After the merge-1 deploy the editor is gone, so reversal needs Route B.

### Route B: sqlite3 over ssh, after the merge-1 deploy (fallback)

Unverified until Peter's run: `fly ssh console` logs in as root by default, the machine can reach the apt mirrors, and the `app` user (`Dockerfile`: `useradd -m -s /bin/bash app`, `USER app`) owns the files in `/app/data`. The runtime image (`debian:bookworm-slim`) carries no `sqlite3` CLI, and the app has no subcommand that runs SQL. Writes go as the `app` user so the WAL and shm files keep their owner; `fly ssh console` has `-u, --user` (confirmed in `fly ssh console --help`), but the apt install needs root.

**B-access** (reach the machine and install).
- **B-access 1.** `fly status -a folio-prmanahan`. **Stop unless exactly one machine is listed**: the volume is per machine, and a second machine holds a second DB.
- **B-access 2.** `fly ssh console -a folio-prmanahan` (root). A stopped machine is started by this (`auto_stop_machines`).
- **B-access 3.** `ls -l /app/data/` and record it. Then `command -v sqlite3 || (apt-get update && apt-get install -y sqlite3)`, and record the apt output and `dpkg -l sqlite3 libsqlite3-0`. The install goes to the machine's root filesystem, not the volume, and is gone at the next restart or deploy. That is intended: no image change, nothing to remember to remove. If the install fails, stop and report; do not copy the DB off the volume or edit it by another route.
- **B-access 4.** Every sqlite3 command below runs as the app user: `runuser -u app -- sqlite3 ...` from the root shell (or reconnect with `fly ssh console -u app -a folio-prmanahan`, if the sqlite3 binary installed in B-access 3 is still present). Never run sqlite3 as root against this DB.

**B-steps** (inside the ssh shell).
- **B1. Record before.** The one-column backup, no content:
  ```
  runuser -u app -- sqlite3 -header -column /app/data/site.db "SELECT id, slug, published FROM articles ORDER BY id;"
  ```
  Note the newest volume snapshot ID (`fly volumes snapshots list <volume id> -a folio-prmanahan`), as merge 2's pre-checks do. Then identify the row; do not guess an id:
  ```
  runuser -u app -- sqlite3 -header -column /app/data/site.db "SELECT id, slug, title, published FROM articles WHERE title LIKE '%Workflow Didn%t Change%';"
  ```
  The `%` between `Didn` and `t` matches a straight or a curly apostrophe. **Expect exactly one row, with `published` = 1. Stop if there are zero rows, two or more rows, or `published` is already 0.** Zero or many means the title or the DB is not what this step assumes; already 0 means it was done, so go to B4. Record the row: its `id` is `<ID>` below, and its `published` value is the reversal's target.
- **B2. Positive control (optional).** As A2.
- **B3. Update**, scoped to the id from B1:
  ```
  runuser -u app -- sqlite3 -cmd ".timeout 5000" /app/data/site.db "UPDATE articles SET published = 0 WHERE id = <ID>; SELECT changes();"
  ```
  Expect `1`. Any other value: stop. A "database is locked" error is safe to retry: nothing was committed.
- **B4. Verify the row:**
  ```
  runuser -u app -- sqlite3 -header -column /app/data/site.db "SELECT id, slug, title, published FROM articles WHERE id = <ID>;"
  ```
  Expect the same row with `published` = 0. This read is the decisive check, for the reason given in A5.
- **B5. Verify the prompt from outside:** as A5's second half (the same question to the chat, and a job-fit analysis).
- **B6. Check ownership, then leave.** `ls -l /app/data/` again. **Stop and report if any `site.db*` file changed owner** from the B-access 3 listing; do not proceed to close the step. Then `exit`. Nothing else to clean up.

**Route B reversal.** In the same kind of shell (repeat B-access 1 to 4):
```
runuser -u app -- sqlite3 -cmd ".timeout 5000" /app/data/site.db "UPDATE articles SET published = 1 WHERE id = <ID>; SELECT changes();"
```
Expect `1` (a "database is locked" error is safe to retry), then re-run B4's SELECT and expect `published` = 1, then repeat the B6 ownership check. This puts the article back in the prompts. Do not reverse it for a rollback unless that is the intent.

**Relative to the rollback blocks below.** The unpublish is data on the volume, so a rollback (merge 1 or merge 2) does not undo it. After a merge-1 rollback the old release serves `/articles` again, and it should no longer list or serve this one (expected from the `published` flag; not checked). Reverse it only if Peter wants the old page back, and know that doing so re-exposes the text to the prompts.

## Rollback, merge 1

**This section applies only until merge 2 deploys.** After that, use "Rollback, merge 2".

**Rollback, merge 1.** `fly.toml` still runs `./folio check-projects` after merge 1.
- **Primary:** revert through `main`, which deploys and fetches a fresh list in CI. It needs GitHub up. The revert must cover all of merge 1's commits, which are the commits PR #80 landed. Merge 1 lands by rebase-merge, so their SHAs on main differ from the branch's: take the range from the PR's merge record, not from branch history.
- **Fast path.** Recorded image, verbatim: `registry.fly.io/folio-prmanahan:deployment-01M7TFD33NVDSNFFY340FW1KA9` (release v81, re-recorded 2026-11-20, confirmed running by `fly status`). Re-run `fly releases --image -a folio-prmanahan` immediately before the merge; the T6 report carries the final value.
  - **Trigger:** the CI Deploy job fails, or any post-deploy check is red.
  - **Run**, from a checkout root that holds `fly.toml`: `fly deploy --image <recorded image> --skip-release-command -a folio-prmanahan`. The recorded image's baked `projects.json` is past its 30-minute bound, so without that flag the release command refuses the deploy.
  - **Post-check:** `fly releases --image -a folio-prmanahan` shows a new top release with the recorded image. Then run `curl -sS -o /dev/null -w '%{http_code}\n' https://peter.manahan.io/api/health` (this wakes the auto-stopped machine; `fly status` may show it stopped), expecting `200`, and `curl -sS -o /dev/null -w '%{http_code}\n' https://peter.manahan.io/articles`, expecting `200` from the old release. `--fail` is not used: it exits 0 on a 3xx, so the printed code is the check. A `301` on `/articles` means the new release is still serving.
- **After either,** the old release serves its own `/articles` pages again.
- **Rolled-back state (fast path).** `main` still contains merge 1 and CI deploys every push to main. Merge nothing else until the revert PR is open and merged. A later `fly deploy --image <recorded image>` without `--skip-release-command` fails the release command on the stale baked list. This is expected and safe.

## Rollback, merge 2

**Rollback, merge 2.**
- **Primary:** redeploy merge 1's recorded image with `fly deploy --image <recorded image> -a folio-prmanahan`, from a post-merge-2 checkout. A pre-merge-2 checkout's `fly.toml` still runs `./folio check-projects`, and the image's baked `projects.json` is past its 30-minute bound, so the release command refuses the deploy.
  - **Recorded image, verbatim:** `registry.fly.io/folio-prmanahan:deployment-01M7WSJ49A5MFBY90G3GZCF5KE` (release v82, merge 1, read 2026-11-24). Re-run `fly releases --image -a folio-prmanahan` immediately before the merge; the T18 report carries the final value. Do not use the v81 image in the merge-1 section.
  - **Post-check:** `fly releases --image -a folio-prmanahan` shows a new top release with that image. Then `curl -sS -o /dev/null -w '%{http_code}\n' https://peter.manahan.io/api/health` returns `200` (this wakes the auto-stopped machine), and `curl -sS -o /dev/null -w '%{http_code}\n' https://peter.manahan.io/api/lists/freshness` returns `404`.
- **Slow path:** revert merge 2 through `main`, which deploys. It needs GitHub up, because the reverted code fetches the list in CI.
- **After either,** the old release ignores the `live_lists` table and serves its own baked list, and its prompt reads the `articles` table. `/api/lists/freshness` answers the JSON 404, so the daily check is red every day; that is expected.
- **Rolled-back state (image rollback).** `main` still contains merge 2 and CI deploys every push to main. Merge nothing else until the revert PR is open and merged: the next push to main redeploys merge 2 through CI and undoes the rollback.
- **Rolling back for bad content:** set `LIVE_LISTS_DISABLED` for that list before rolling forward. Otherwise the poisoned row loads and is served from the first request.

**Triage: a red post-deploy check after merge 2.** Run `fly logs -a folio-prmanahan --no-tail`. If a `live lists refresh` line shows `requests=` containing `:403` or `:429`, or `ratelimit_remaining=` ending `:0`, the cause is the parked GitHub rate budget (Frame FQ-2), not a defect, and the check will go green once the budget resets and a refresh succeeds. The check's own output shows only `RED: <list> has no time` with the freshness status, so the log line is how to tell. Fly's egress to GitHub was not measured, so this rule rests on the log line alone.

## Break-glass runbook

**Break-glass runbook** (final wording; steps locked at Frame D-10):
1. `fly secrets set LIVE_LISTS_DISABLED=<projects|articles|projects,articles> -a folio-prmanahan`. If the machine does not restart on its own, restart it. At start the server deletes that list's stored row.
2. First wake the machine: `curl -sS -o /dev/null -w '%{http_code}\n' https://peter.manahan.io/api/health`, expecting `200`. Then read the start line: `fly logs -a folio-prmanahan --no-tail | grep 'live lists start'`. **Stop unless both hold:**
   - The start log line names the list with outcome `deleted` or `absent`. If it says `failed`, do not go on to step 5: the bad row is still stored.
   - `https://peter.manahan.io/api/lists/freshness` reports that list as `null`. For projects, `/api/lists/projects` also answers 503.

   If the list is not named, check the secret's name. A misspelled or miscased value names both lists and deletes the other list's row too; step 5's refresh restores it.
3. While the switch is set, the daily check goes red, and so does the post-deploy check on every deploy. That is the switch working. A misspelled value disables both lists and logs at `error`.
4. Correct the text at its source: the GitHub description, or the mnemra.dev post.
5. `fly secrets unset LIVE_LISTS_DISABLED -a folio-prmanahan`, restarting if needed. Confirm the list's time in the freshness read is later than the unset. If freshness reads `null` right after the restart, re-read for up to 150 s before treating it as a failure.
