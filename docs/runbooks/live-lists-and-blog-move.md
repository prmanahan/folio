# Live lists and blog move: rollback and break-glass runbook

Source: `docs/specs/2026-11-18-live-lists-and-blog-move.md`, blob `4e8af5ed0aa02d44aa01aafbaad9d861070c5705`. The break-glass runbook and the merge-2 rollback apply only once merge 2 has deployed.

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
