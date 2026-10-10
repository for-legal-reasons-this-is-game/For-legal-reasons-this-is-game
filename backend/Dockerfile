# ---- Build stage ----
# Build context is the repo root (needs ../proto).
FROM rust:1-slim-bookworm AS builder

# protobuf-compiler provides protoc for proto/build.rs; libprotobuf-dev
# provides the well-known types (google/protobuf/timestamp.proto).
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/proto
COPY proto/Cargo.toml proto/build.rs proto/*.proto ./
COPY proto/src ./src

WORKDIR /app/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src ./src
COPY backend/migrations ./migrations

RUN cargo build --release --locked --bin backend

# ---- Runtime stage ----
# No Infisical CLI: secrets arrive as env (e.g. K8s Secret via envFrom).
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home backend

WORKDIR /app
COPY --from=builder /app/backend/target/release/backend ./backend
RUN chown backend:backend ./backend

USER backend
EXPOSE 8000
CMD ["./backend"]
