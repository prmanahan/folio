# Project Standards

General architecture and engineering standards applied to this project.
These were projected on 2026-04-05 and may be overridden by project-specific
decisions (P-*.md files in this directory).

---

## G-0001: Testing Philosophy

90% code coverage threshold (lines and functions) enforced by default. Exceptions must be documented with explicit reasoning. Inverted test pyramid: far more unit tests (with mocks) than integration tests. Integration tests only verify actual integration seams. All projects held to professional standards regardless of audience.

---

## G-0002: No Compile-Time Asset Embedding

Do not use `rust_embed` or similar compile-time asset embedding in containerized projects. Serve static files at runtime (`tower_http::ServeDir` or equivalent). Startup validation ensures the static directory exists. Docker multi-stage builds copy both binary and frontend assets into the final image. Backend builds, tests, and coverage runs must be independent of frontend builds.

---

## G-0003: Two-Tier Architecture Decision Records

General standards live in this DEFAULTS.md file, projected once and owned by the project. Project-specific decisions go in P-NNNN-*.md files in this directory. If a project-specific decision overrides a default, it says so explicitly in its status field.
