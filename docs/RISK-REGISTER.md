# Risk register

Each entry is a security risk the maintainer accepted rather than fixed. An entry records who signed, when, the task it was accepted under, and what would make us revisit it. An entry is not closed by being old: it stays until one of its revisit conditions fires and the risk is fixed or re-accepted.

## RISK-1: CSP guard's charset rule sees only the parsed tree

Warden #4710 round 7, R7-F1.

The build-time CSP guard (`frontend/scripts/csp-lib.mjs`) rejects a non-UTF-8 `<meta charset>` or `http-equiv` content-type only where jsdom's tree exposes it; one inside `<template>` or `<title>` is not seen.

- **Mitigation:** the server sends `Content-Type: text/html; charset=utf-8` on static HTML (`libs/site-core/static_files.rs`), which outranks an in-document charset declaration, and the post-deploy check in `.github/workflows/deploy.yml` fails if that charset is missing.
- **Actor needed:** a repo committer or a build dependency; not reachable by anyone typing into the chat.
- **Accepted by:** Peter Manahan, 2026-11-11, task #4710.
- **Revisit if:** the server stops sending the charset, or the deploy check's charset assertion is removed.

## RISK-2: CSP header falls back to the minimal policy if its file is unusable

Warden #4710 round 7, R7-F3.

If `build/csp-header.txt` is missing, empty or not a valid header value at startup, the server logs one ERROR and serves `STATIC_CSP` (`frame-ancestors 'none'; object-src 'none'; base-uri 'none'`, no script directive) on static responses. The page then relies on its meta CSP alone, which does not cover markup placed before the meta (#4710 round 6, R6-F1).

- **Detection:** the post-deploy check fails because the header no longer equals the page's policy plus `frame-ancestors`, but it runs after the new release takes traffic; the ERROR log line has no alert.
- **Production build path:** the Dockerfile runs `npm run build && npm run check:csp` in one step, which always writes the file.
- **Actor needed to exploit the fallback:** a repo committer or a build dependency.
- **Accepted by:** Peter Manahan, 2026-11-11, task #4710, over the alternative of refusing to start without the file (rejected: Fly runs one machine, and whether it keeps the old release serving when a new one crashes on start is unverified).
- **Revisit when:** the post-deploy CSP check fails, when log alerting is added, or when the deploy strategy is known to keep the old release on a failed start.

## RISK-3: one Cloudflare account fronts both mnemra.dev and peter.manahan.io

Warden #4853 Frame review round 1, W-1 (TH-20).

mnemra.dev and peter.manahan.io sit in the same Cloudflare account (Peter, 2026-11-18). Whoever holds that account can read `EDGE_AUTH_TOKEN`, which Cloudflare injects as `X-Folio-Edge-Auth` (`libs/site-core/middleware/origin_lock.rs`), and can repoint folio's DNS and edge rules. The exposure predates #4853. The live-lists design brings it into scope because mnemra.dev's feed text now reaches folio's chat at runtime, so the same account also steers that text.

- **Actor and cell:** ACTOR-4 (a holder of Peter's credentials, widened to the Cloudflare account). Warden rated it medium/high, above the bar signed 2026-11-18.
- **Where:** `docs/intent/live-lists-and-blog-move-frame.md`, TH-20, at EE-4 (Cloudflare edge) and the flows through it.
- **Compensating controls, held by Peter:**
  - Hardware-key 2FA on the Cloudflare account. This is the only control that works against the account holder, and it gates interactive login only. API tokens bypass it (Warden Frame round 3, R3-N-2a; from reviewer knowledge, not checked against Cloudflare's docs). Peter kept the "Cloudflare Agent Token - 2026-04-23" (see the baseline below). If that token holds API-token edit access, its holder can mint tokens with any of Peter's permissions, and no listed control stops that path.
  - A deploy token for mnemra.dev, scoped to that zone only.
  - A rotation path for `EDGE_AUTH_TOKEN`. Rotation limits a leaked token, but not the account holder, who can read the new one. No written runbook exists yet (search for "rotat" with EDGE_AUTH over `docs` and `*.md` at 4a445a5 found nothing).
- **Accepted by:** Peter Manahan, 2026-11-18, task #4853 ("Same account, accept"), over the alternative of moving mnemra.dev to its own Cloudflare account (rejected: account admin work outside folio). He re-affirmed it the same day ("Re-affirm") after being told that 2FA does not gate API tokens. Over the recommendation to revoke the agent token, he chose "Keep, record it".
- **Revisit when:**
  - Either domain moves accounts. This announces itself.
  - Any of the three compensating controls is found absent. This is **parked**, not self-announcing: Peter checks all three as part of the Frame's PK-1 monthly review.
  - The Cloudflare account gains a second member or an API token beyond this baseline. Peter recorded the baseline on 2026-11-18 from the Cloudflare dashboard ("just me"):
    - Members: Peter only.
    - API tokens:
      - "Cloudflare Agent Token - 2026-04-23", permissions `User.API Tokens, User.Memberships`, shown as last used 6mo ago. Kept by Peter. Its access level (Read or Edit) and where it is stored are not recorded; PK-1's next run records both.
      - "mnemra build token", permissions `Zone.Workers Routes, Zone.SSL and Certificates`, shown as last used 2mo ago. Zone scope: mnemra.dev only (Peter, 2026-11-18). Access level not recorded.

    PK-1 compares the live list against this baseline.
- **Recorded:** 2026-11-18. **Next review:** the first PK-1 monthly check after the Frame locks.
