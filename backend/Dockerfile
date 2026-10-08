# Runtime-only image: expects `make compile_backend` to have produced
# backend/target/debug/backend (gnu, distro dynamic linker via flake RUSTFLAGS).
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home backend

# Official CLI image ships a static binary; copy it instead of apt-installing.
COPY --from=infisical/cli:latest /bin/infisical /usr/local/bin/infisical

WORKDIR /app

# Build context is the repo root (see compose.yaml).
COPY backend/target/debug/backend ./backend
COPY backend/entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x ./backend /usr/local/bin/entrypoint.sh \
    && chown backend:backend ./backend

USER backend
EXPOSE 8000
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
CMD ["./backend"]
