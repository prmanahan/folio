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
