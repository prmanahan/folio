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

# Format, lint, test — pre-commit check
check:
    cargo fmt
    cargo clippy -- -D warnings
    cargo test

# Build release binary
release: frontend-build
    cargo build --release

# Docker build and run
docker:
    docker compose up --build
