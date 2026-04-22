# Project Standards

General architecture and engineering standards applied to this project.
These were projected on 2026-04-21 and may be overridden by project-specific
decisions (P-*.md files in this directory).

---

## G-0001: Testing Philosophy — 90% Coverage, Inverted Pyramid

90% code coverage threshold (lines and functions) enforced by default. Exceptions must be documented with explicit reasoning. Inverted test pyramid: far more unit tests (with mocks) than integration tests. Integration tests only verify actual integration seams. All projects held to professional standards regardless of audience.

---

## G-0002: No Compile-Time Asset Embedding

Do not use `rust_embed` or similar compile-time asset embedding in Rust projects deployed via Docker containers. Serve static files at runtime (`tower_http::services::ServeDir` or equivalent). Startup validation ensures the static directory exists. Docker multi-stage builds copy both binary and frontend assets into the final image. Backend builds, tests, and coverage runs must be independent of frontend builds.

---

## G-0003: Two-Tier Architecture Decision Records

General standards live in this DEFAULTS.md file, projected once and owned by the project. Project-specific decisions go in P-NNNN-*.md files in this directory. If a project-specific decision overrides a default, it says so explicitly in its status field.

---

## G-0004: Hybrid XML Format for Agent Profiles

All team agent profiles use hybrid XML format: XML tags for structural sections (`<role>`, `<persona>`, `<principles>`, etc.), markdown preserved inside tags for prose. Maximum two levels of nesting. Profiles stay under 4KB; overflow goes into skill files.

---

## G-0005: SQL file-based migrations for embedded SQLite

Use SQL file-based migrations for Rust projects with embedded SQLite. Migrations live in `migrations/NNNN_<description>.sql` and are embedded into the binary via `include_str!()`. A generic migration runner reads `schema_version` and applies missing migrations in order. Idempotent SQL where possible; version-tracking table guards non-idempotent operations. Forward-only — no down migrations.

---

## G-0006: Justfile as CI Contract

CI YAML invokes `just ci` and nothing else; gate logic lives in the per-project justfile. Fixed recipe names: `verify-test`, `verify-lint`, `verify-type`, `verify-coverage`, `verify-build`, `verify-smoke`, composite `ci`. Each `verify-*` recipe emits a last-line `GATE <name> <PASS|FAIL> <detail>` for human legibility; the exit code is the source of truth. `verify-*` recipes do not run `--fix` — those belong under `fix-*`. All recipes are idempotent from any cwd.
