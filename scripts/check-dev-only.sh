#!/usr/bin/env bash
# Holds one package to dev-only: absent from the shipped (normal + build) dependency graph.
# Usage: scripts/check-dev-only.sh <package>
#
# Two controls rest on this. deny.toml's ignore for RUSTSEC-2026-0543 suppresses the advisory
# by ID across the whole graph, so it is sound only while chrono stays out of the shipped graph
# (#4851). rcgen, the TLS test harness's certificate generator, is test-only by Peter's ruling
# (live-lists spec, Decision levels "TLS test harness"), and it must never reach the binary.
#
#   package absent from normal+build graph -> exit 0 (dev-only; the control holds)
#   package in the shipped graph           -> exit 1 (the control no longer holds)
#   package not in the lockfile at all     -> exit 1 (remove this guard step and any ignore that depends on it)
#   no package argument, or more than one  -> exit 1
#   any other cargo failure                -> exit 1 (never passes on an unknown error)
#
# `--target all` so a target-conditional dependency cannot hide.
set -u

if [ "$#" -ne 1 ] || [ -z "$1" ]; then
  echo "::error::usage: scripts/check-dev-only.sh <package> (exactly one package name); failing rather than guarding nothing."
  exit 1
fi
pkg="$1"
out="$(mktemp)"
err="$(mktemp)"
trap 'rm -f "$out" "$err"' EXIT

cargo tree --locked --target all -e normal,build -i "$pkg" >"$out" 2>"$err"
rc=$?

if [ "$rc" -eq 0 ]; then
  if [ -s "$out" ]; then
    echo "::error::$pkg is in the shipped dependency graph. It is held to dev-only, and anything that assumed so (for chrono, the RUSTSEC-2026-0543 ignore in deny.toml) no longer holds."
    cat "$out"
    exit 1
  fi
  echo "ok: $pkg is not in the normal/build dependency graph (dev-only)."
  exit 0
fi

if grep -q "did not match any packages" "$err"; then
  echo "::error::$pkg is no longer in Cargo.lock. Remove this guard step (the step running 'scripts/check-dev-only.sh $pkg' in .github/workflows/test.yml and deploy.yml) and any ignore that depends on it (for chrono, the RUSTSEC-2026-0543 ignore in deny.toml)."
  cat "$err"
  exit 1
fi

echo "::error::cargo tree failed (exit $rc) for a reason this check does not recognise; failing rather than passing."
cat "$err"
exit 1
