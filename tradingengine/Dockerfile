FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    protobuf-compiler \
    libprotobuf-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app/proto
COPY proto/Cargo.toml proto/build.rs proto/*.proto ./
COPY proto/src ./src

WORKDIR /app/tradingengine
COPY tradingengine/Cargo.toml tradingengine/Cargo.lock ./
COPY tradingengine/src ./src

RUN cargo build --release --locked --bin tradingengine

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

RUN useradd --system --create-home tradingengine

WORKDIR /app
COPY --from=builder /app/tradingengine/target/release/tradingengine ./tradingengine
COPY --from=ghcr.io/grpc-ecosystem/grpc-health-probe:v0.4.57 /ko-app/grpc-health-probe /usr/local/bin/grpc_health_probe

USER tradingengine
EXPOSE 50051
CMD ["./tradingengine"]
