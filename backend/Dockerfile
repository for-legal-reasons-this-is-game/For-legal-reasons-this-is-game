# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/* \
    && cargo install cargo-watch

WORKDIR /app

# Cache dependencies layer
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo 'fn main() {}' > src/main.rs && cargo build --locked && rm -rf src

COPY src ./src
COPY migrations ./migrations

RUN cargo build --locked

# ---- Runtime stage ----
FROM rust:1-slim-bookworm AS runtime

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
COPY --from=builder /app/target/debug/backend ./backend
COPY --from=builder /usr/local/cargo/bin/cargo-watch /usr/local/bin/cargo-watch
COPY --from=builder /app/src ./src
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/Cargo.toml /app/Cargo.lock ./
COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh && chown -R backend:backend /app

USER backend
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["sh", "-c", "cargo watch -x run"]
