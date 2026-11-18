#!/usr/bin/env bash
# #4851: the deny.toml ignore for RUSTSEC-2026-0543 suppresses the advisory by
# ID across the whole graph, so it is only sound while chrono stays out of the
# shipped (normal + build) dependency graph. This is the mechanism that holds
# that, and the one that tells us when the ignore can go.
#
#   chrono absent from normal+build graph -> exit 0 (dev-only; ignore holds)
#   chrono in the shipped graph           -> exit 1 (ignore no longer holds)
#   chrono not in the lockfile at all     -> exit 1 (remove the ignore and this check)
#   any other cargo failure               -> exit 1 (never passes on an unknown error)
#
# `--target all` so a target-conditional dependency cannot hide. The optional
# argument names a different package; it exists only to exercise each branch.
set -u

pkg="${1:-chrono}"
out="$(mktemp)"
err="$(mktemp)"
trap 'rm -f "$out" "$err"' EXIT

cargo tree --locked --target all -e normal,build -i "$pkg" >"$out" 2>"$err"
rc=$?

if [ "$rc" -eq 0 ]; then
  if [ -s "$out" ]; then
    echo "::error::$pkg is in the shipped dependency graph. The RUSTSEC-2026-0543 ignore in deny.toml assumed it was dev-only and no longer holds."
    cat "$out"
    exit 1
  fi
  echo "ok: $pkg is not in the normal/build dependency graph (dev-only)."
  exit 0
fi

if grep -q "did not match any packages" "$err"; then
  echo "::error::$pkg is no longer in Cargo.lock. Remove the RUSTSEC-2026-0543 ignore from deny.toml, and remove this check (scripts/check-chrono-dev-only.sh and its step in .github/workflows/test.yml)."
  cat "$err"
  exit 1
fi

echo "::error::cargo tree failed (exit $rc) for a reason this check does not recognise; failing rather than passing."
cat "$err"
exit 1
