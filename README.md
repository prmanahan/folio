# folio

Personal portfolio and resume site. Rust/Axum backend with a SvelteKit frontend embedded into the binary at compile time via rust-embed.

## Tech Stack

- **Backend**: Rust (edition 2024), Axum 0.8
- **Frontend**: SvelteKit (Svelte 5), Tailwind CSS v4, DaisyUI v5
- **Database**: SQLite via rusqlite
- **AI features**: rig-core (Anthropic Claude), optional
- **Embedding**: rust-embed (SPA baked into the binary at compile time)

## Prerequisites

- **Rust** — edition 2024
- **Node.js** — v22+
- **just** — command runner (`cargo install just` or `brew install just`)
- **Docker** — optional, for containerized runs

## Getting Started

```bash
git clone <repo-url>
cd folio
cp .env.example .env
```

Edit `.env`:

- `ADMIN_PASSWORD` — **required**, no default. The app will panic at startup without it.
- `ANTHROPIC_API_KEY` — optional. Leave blank to disable AI chat and job-fit analysis features.
- `CORS_ORIGIN` — defaults to `http://localhost:3000`. No change needed for local dev.

## Running Locally

### Full stack (recommended)

Builds the frontend and starts the backend:

```bash
just run
```

The server starts on port 3000 (or whatever `PORT` is set to in `.env`). Open `http://localhost:3000`.

### Frontend hot-reload

In a separate terminal, run the Vite dev server for instant UI feedback:

```bash
just frontend-dev   # serves on port 5173
```

The frontend proxies API calls to `http://localhost:3000`, so the backend must be running (`cargo run` in another terminal).

### Docker

```bash
just docker
```

Or manually: `docker compose up --build`. App at `http://localhost:8080`.

## Database

SQLite database. Migrations run automatically at startup — no manual step needed.

The DB file lives at `data/site.db` (gitignored).

**Seed with demo data:**

```bash
just seed
```

## Running Tests

```bash
just test      # backend unit and integration tests
just e2e       # end-to-end tests (requires app on localhost:8080)
just check     # fmt + clippy + test (pre-commit)
```

## Project Structure

```
cmd/           Rust binary — Axum server entry point
libs/          Rust libraries — core logic, DB, AI, models, routes
frontend/      SvelteKit frontend — Svelte 5, Tailwind v4, DaisyUI v5
data/          SQLite database and seed SQL
migrations/    SQL schema migrations
e2e/           Playwright end-to-end tests
```

## Environment Variables

| Variable | Required | Default | Description |
|---|---|---|---|
| `PORT` | No | `3000` | Port the server listens on |
| `DATABASE_URL` | No | `data/site.db` | Path to the SQLite database file |
| `ADMIN_PASSWORD` | **Yes** | — | Password for the admin dashboard |
| `CORS_ORIGIN` | No | `http://localhost:3000` | Allowed CORS origin |
| `ANTHROPIC_API_KEY` | No | — | Anthropic API key for AI features |
