# Stage 1: Build frontend
FROM node:22-slim AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
#
# Pinned to the same version as rust-toolchain.toml's `channel`. This is folio's
# third Rust build surface (local, CI, image) and the only one that produces the
# binary served at peter.manahan.io -- `flyctl deploy --remote-only` builds from
# this file. It sat on 1.88 while the other two moved to 1.98.0 (#3479), which
# let a 1.89+ feature pass `just check` and CI green and then fail at deploy.
#
# `1.98.0-bookworm`, not `1.98-bookworm`: the floating minor tag rolls to 1.98.1
# on the next patch release and silently re-opens that divergence.
#
# `bookworm`, not `trixie`: stage 3 runs on debian:bookworm-slim. A trixie build
# stage links against a newer glibc than the runtime image provides, and the
# binary fails to start rather than failing to build.
#
# NOT fixed by adding `COPY rust-toolchain.toml ./`: an older base would then
# auto-download the pinned toolchain at image-build time, changing the deployed
# compiler as a side effect and lengthening every build. The image tag is the
# mechanism.
#
# Deliberately not digest-pinned. `rust:1.98.0-bookworm@sha256:...` is valid and
# names both, so the tag would stay greppable -- the objection is not syntax.
# Official images are rebuilt under the same tag whenever Debian ships a security
# update, so a digest here goes stale within weeks, and what moves under it is the
# Debian package set rather than rustc. Nothing in this repo would refresh it:
# there is no .github/dependabot.yml, and the dependency PRs here come from
# settings-level security updates only. A pinned digest would therefore mean
# building on unpatched Debian until a human noticed by hand. We take drift in the
# build-stage package set over sitting on known-unpatched Debian; the tag still
# pins rustc, which is what this file exists to do.
FROM rust:1.98.0-bookworm AS rust-builder
WORKDIR /app

# Cache dependencies: copy manifests, create dummy sources, build deps
#
# `--locked` on both release builds: the workspace declares caret ranges
# (`axum = "0.8"`, `tokio = "1"`), so Cargo.lock is the only thing actually
# pinning dependency versions in the shipped binary. Without it, cargo
# silently re-resolves when the lock and the manifests disagree instead of
# failing loud -- which also means a `cargo audit` result would only
# conditionally describe what deployed.
COPY Cargo.toml Cargo.lock ./
COPY cmd/server/Cargo.toml cmd/server/Cargo.toml
COPY libs/site-core/Cargo.toml libs/site-core/Cargo.toml
RUN mkdir -p cmd/server libs/site-core \
    && echo 'fn main() {}' > cmd/server/main.rs \
    && echo '' > libs/site-core/site_core.rs
RUN cargo build --locked --release
RUN rm -rf cmd/server/main.rs libs/site-core/site_core.rs

# Copy real source + frontend build output
COPY cmd/ cmd/
COPY libs/ libs/
COPY migrations/ migrations/
# Touch source files so Cargo sees them as newer than the cached dep artifacts
RUN find cmd libs -name "*.rs" -exec touch {} +

# Build real binary
RUN cargo build --locked --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash app
USER app
WORKDIR /app

COPY --from=rust-builder /app/target/release/folio ./
COPY --from=frontend-builder /app/frontend/build/ /app/frontend/build/

ENV PORT=8080
ENV STATIC_DIR=/app/frontend/build
ENV DATABASE_URL=/app/data/site.db
# Override in production with a unique, secret value
ENV PAGE_HIT_SALT=change-me-in-production
EXPOSE 8080

CMD ["./folio"]
