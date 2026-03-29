# Filesystem Structure

```
folio/
├── cmd/
│   └── server/          # HTTP server binary (Axum)
│       ├── main.rs
│       └── Cargo.toml
├── libs/
│   └── site-core/       # Core library — models, routes, DB, AI
│       ├── lib.rs
│       ├── ai/
│       ├── auth.rs
│       ├── config.rs
│       ├── db/
│       ├── error.rs
│       ├── models/
│       ├── routes/
│       ├── state.rs
│       ├── static_files.rs
│       └── Cargo.toml
├── frontend/            # SvelteKit frontend
├── migrations/          # SQL schema migrations
├── data/                # SQLite database and seed data
├── e2e/                 # Playwright end-to-end tests
├── justfile             # Common commands
└── Cargo.toml           # Workspace root
```

## Rules

- Flat structure: binaries in `cmd/`, libraries in `libs/`. No `src/` directories.
- Cargo workspace manages shared metadata (version, edition, authors, license).
- Edition: Rust 2024.
- All dependencies declared at workspace level, referenced via `workspace = true`.
