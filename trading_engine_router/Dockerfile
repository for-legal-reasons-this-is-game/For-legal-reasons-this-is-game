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

WORKDIR /app/trading_engine_router
COPY trading_engine_router/Cargo.toml trading_engine_router/Cargo.lock ./
COPY trading_engine_router/src ./src
RUN cargo build --release --locked --bin trading_engine_router

# ---- Runtime stage ----
# No Infisical CLI: secrets arrive as env (e.g. K8s Secret via envFrom).
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home router

WORKDIR /app
COPY --from=builder /app/trading_engine_router/target/release/trading_engine_router ./router
RUN chown router:router ./router

USER router
EXPOSE 50051
CMD ["./router"]
