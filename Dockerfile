# Stage 1: Build frontend
FROM node:22-slim AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Build Rust binary
FROM rust:1.88-bookworm AS rust-builder
WORKDIR /app

# Cache dependencies: copy manifests, create dummy sources, build deps
COPY Cargo.toml Cargo.lock ./
COPY cmd/server/Cargo.toml cmd/server/Cargo.toml
COPY libs/site-core/Cargo.toml libs/site-core/Cargo.toml
RUN mkdir -p cmd/server libs/site-core \
    && echo 'fn main() {}' > cmd/server/main.rs \
    && echo '' > libs/site-core/lib.rs
RUN cargo build --release
RUN rm -rf cmd/server/main.rs libs/site-core/lib.rs

# Copy real source + frontend build output
COPY cmd/ cmd/
COPY libs/ libs/
COPY migrations/ migrations/
COPY --from=frontend-builder /app/frontend/build/ frontend/build/

# Touch source files so Cargo sees them as newer than the cached dep artifacts
RUN find cmd libs -name "*.rs" -exec touch {} +

# Build real binary (rust-embed reads frontend/build/ at compile time)
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash app
USER app
WORKDIR /app

COPY --from=rust-builder /app/target/release/folio ./

ENV PORT=8080
ENV DATABASE_URL=/app/data/site.db
EXPOSE 8080

CMD ["./folio"]
