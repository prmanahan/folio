# Build & Run Commands

```bash
just run                 # build frontend + start backend
just frontend-dev        # frontend hot-reload on :5173
just test                # cargo test
just check               # fmt + clippy + test
just release             # release build
just docker              # docker compose up --build
just seed                # load demo data
```

## Verification checklist

Before committing, run:
1. `just check`
