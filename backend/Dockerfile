# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations

RUN cargo build --release --locked

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

# ca-certificates for TLS, curl to install the Infisical CLI, which the
# entrypoint uses to pull secrets at startup.
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl gnupg \
    && curl --proto '=https' --tlsv1.2 -fsSL https://artifacts-cli.infisical.com/infisical.gpg \
        | gpg --dearmor -o /usr/share/keyrings/infisical-archive-keyring.gpg \
    && chmod 644 /usr/share/keyrings/infisical-archive-keyring.gpg \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/usr/share/keyrings/infisical-archive-keyring.gpg] https://artifacts-cli.infisical.com/deb stable main" \
        > /etc/apt/sources.list.d/infisical.list \
    && apt-get update && apt-get install -y --no-install-recommends infisical \
    && apt-get purge -y gnupg && apt-get autoremove -y \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home backend

WORKDIR /app
COPY --from=builder /app/target/release/backend ./backend
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

USER backend
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["./backend"]
