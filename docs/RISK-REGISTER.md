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
