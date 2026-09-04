# Personal Site — common commands

# Default: show available recipes
default:
    @just --list

# Build frontend (required before cargo build/run)
frontend-build:
    cd frontend && npm install && npm run build

# Run the backend (frontend must be built first)
run: frontend-build
    cargo run

# Run frontend dev server with hot-reload (hit localhost:5173)
frontend-dev:
    cd frontend && npm run dev

# Run all backend tests
test:
    cargo test

# Run end-to-end tests (requires app on localhost:8080)
e2e:
    npx playwright test

# Apply rustfmt across the workspace — the writing half of `check`'s
# `fmt --check`, so a red gate has a one-command fix.
fmt:
    cargo fmt --all

# Format check, lint, test — pre-commit gate.
#
# `fmt --check` reports, it does not rewrite: a gate whose first step
# edits the tree cannot fail, and its churn rides into whatever branch is
# checked out. Run `just fmt` to apply.
#
# `--all-targets` puts tests, benches and examples under the same lint
# bar as the library; without it no test target is ever linted.
#
# Deterministic only because rust-toolchain.toml pins the toolchain — on a
# floating `stable`, `fmt --check` goes red on every rustfmt release.
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test

# Build release binary
release: frontend-build
    cargo build --release

# Docker build and run
docker:
    docker compose up --build

# --- Coverage ----------------------------------------------------------------
#
# These recipes MEASURE and REPORT. Nothing here fails a build on a coverage
# figure (ruling, #2750): the 90% in CLAUDE.md / G-0001 is a target, and a
# threshold flag set below current coverage passes unconditionally while
# reading like enforcement. If a real gate is wanted later it is a separate
# decision, not a flag quietly re-added here.
#
# HOST REQUIREMENT: `cargo llvm-cov` comes from the cargo-llvm-cov binary,
# installed with `cargo install cargo-llvm-cov`. Nothing declares or pins it —
# not Cargo.lock, not rust-toolchain.toml (which supplies only the llvm-tools
# component these recipes build on). A machine without it gets cargo's own
# "no such subcommand" error, which is legible, but no CI job runs these
# recipes today so that path has never been exercised.
#
# Numbers below were measured with cargo-llvm-cov 0.8.6. The version matters.
# MEASURED on 0.8.6: no libs/site-core/tests/ file appears in the report at
# all, and adding `--ignore-filename-regex 'tests/'` changes the total by
# 0.00 — which is why no such flag appears here. (Positive control, so that
# is a real absence and not a dead flag: `--ignore-filename-regex 'routes/'`
# moves the denominator 5548 -> 3422 lines.) NOT measured, and therefore not
# claimed: *why* 0.8.6 leaves them out. If a future version starts including
# test sources, they enter the denominator at ~100% covered and the total
# drifts UPWARD.
# ⚠ That drift is UNMONITORED. Nothing runs these recipes on a cadence and
# nothing diffs the number, so no signal fires. Pinning cargo-llvm-cov would
# be the mechanism that catches it; that is a tool-proposal decision, out of
# scope for #2750.

# Collect coverage profile data across every workspace target.
#
# Private on purpose: `backend-coverage` and `backend-coverage-lcov` differ
# only in how they RENDER this profile, so defining what gets measured once
# is what stops the two recipes drifting to different numbers.
#
# `--workspace` with NO `--lib`. `--lib` builds only the library targets and
# so drops everything the 22 integration tests in libs/site-core/tests/ reach.
# Both measured 2026-09-04 on this tree (5d54aff), same instrument:
#
#            line     function   region
#   --lib    64.69%   53.49%     67.40%
#   here     79.06%   68.24%     79.53%
#
# ~14 points on both line and function. The recipe must print the number we
# quote elsewhere, so it measures the full target set.
#
# The leading `clean` is what makes the figure cold-reproducible: stale
# .profraw from an earlier run otherwise merges into the total.
_cov-collect:
    cargo llvm-cov clean --workspace
    cargo llvm-cov --workspace --no-report

# Measure backend coverage: print the per-file table and write the HTML report.
backend-coverage: _cov-collect
    cargo llvm-cov report --html
    cargo llvm-cov report

# Measure backend coverage and write LCOV for external tooling.
backend-coverage-lcov: _cov-collect
    cargo llvm-cov report --lcov --output-path target/llvm-cov/lcov.info
    cargo llvm-cov report

# Measure backend coverage and open the HTML report.
backend-coverage-open: backend-coverage
    open target/llvm-cov/html/index.html

# Discard collected coverage data.
backend-coverage-clean:
    cargo llvm-cov clean --workspace

# `npm ci`, not the `npm install` that `frontend-build` uses: ci restores the
# lockfile exactly and cannot rewrite it, so a coverage run leaves the tree
# clean and the number is reproducible from a cold node_modules. Without an
# install step at all the recipe simply fails on a fresh checkout.
#
# `just --list` renders only the LAST comment line above a recipe, which is why
# the rationale sits in its own block and the one-line doc sits on its own.

# Run frontend unit tests with a coverage report.
frontend-coverage:
    cd frontend && npm ci && npm run test:unit:coverage

# Run frontend coverage and open the HTML report.
frontend-coverage-open: frontend-coverage
    open frontend/coverage/index.html
