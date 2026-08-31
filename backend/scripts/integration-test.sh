#!/usr/bin/env bash
# Spins up a throwaway Postgres container, boots the real backend binary
# against it, runs the integration test suite, then tears everything down.
# Mirrors the `integration` job in .github/workflows/*.yaml.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

CONTAINER_NAME="${CONTAINER_NAME:-backend-integration-pg}"
DB_PORT="${DB_PORT:-5433}"
DATABASE_URL="postgres://postgres:postgres@127.0.0.1:${DB_PORT}/backend"
TB_CLUSTER_ID="${TB_CLUSTER_ID:-0}"
TB_IP_ADRESSES="${TB_IP_ADRESSES:-127.0.0.1:3000}"
export TB_CLUSTER_ID TB_IP_ADRESSES
SERVER_LOG="$(mktemp)"
BACKEND_PID=""

cleanup() {
  if [[ -n "$BACKEND_PID" ]] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    kill "$BACKEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
  fi
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
  rm -f "$SERVER_LOG"
}
trap cleanup EXIT

echo "==> starting postgres ($CONTAINER_NAME on port $DB_PORT)"
docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
docker run -d --name "$CONTAINER_NAME" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=backend \
  -p "${DB_PORT}:5432" \
  postgres:16 >/dev/null

echo "==> waiting for postgres to be ready"
for _ in $(seq 1 30); do
  docker exec "$CONTAINER_NAME" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done

echo "==> building backend"
DATABASE_URL="$DATABASE_URL" cargo build --locked

echo "==> starting backend (logs: $SERVER_LOG)"
DATABASE_URL="$DATABASE_URL" nohup cargo run --locked >"$SERVER_LOG" 2>&1 &
BACKEND_PID=$!

echo "==> waiting for backend to be ready"
ready=false
for _ in $(seq 1 30); do
  if curl -sf http://127.0.0.1:8000/api/v1/users >/dev/null 2>&1; then
    ready=true
    break
  fi
  sleep 1
done
if [[ "$ready" != "true" ]]; then
  echo "backend did not become ready in time" >&2
  cat "$SERVER_LOG" >&2
  exit 1
fi

echo "==> running integration tests"
cargo test --locked --test integration_test
