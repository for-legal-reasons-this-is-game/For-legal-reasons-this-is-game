#!/usr/bin/env bash
# Spins up throwaway Postgres + TigerBeetle containers, boots the real backend
# binary against them, runs the integration test suite, then tears everything
# down. This is the single source of truth for the `integration` job in
# .github/workflows/ci.yaml and backend_ci.yaml (they just call this script
# inside `nix develop`).
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# --- config (all overridable from the environment) ----------------------------
PG_CONTAINER="${PG_CONTAINER:-backend-integration-pg}"
DB_PORT="${DB_PORT:-5433}"
DATABASE_URL="postgres://postgres:postgres@127.0.0.1:${DB_PORT}/backend"

TB_CONTAINER="${TB_CONTAINER:-backend-integration-tb}"
TB_PORT="${TB_PORT:-3000}"
# cn-tigerbeetle 0.1.2 (published 2025-12-12) speaks the protocol of the
# TigerBeetle 0.16.67 release (2025-12-15). If the client is ever bumped and
# the handshake starts failing with "client release too low/high", bump this
# tag to the server release that matches the new client.
TB_IMAGE="${TB_IMAGE:-ghcr.io/tigerbeetle/tigerbeetle:0.16.67}"
TB_CLUSTER_ID="${TB_CLUSTER_ID:-0}"
TB_IP_ADRESSES="${TB_IP_ADRESSES:-127.0.0.1:${TB_PORT}}"
export TB_CLUSTER_ID TB_IP_ADRESSES

SERVER_LOG="$(mktemp)"
BACKEND_PID=""

cleanup() {
  local code=$?
  if [[ $code -ne 0 ]]; then
    echo "==> FAILED (exit $code); dumping logs" >&2
    echo "----- backend log -----" >&2
    cat "$SERVER_LOG" >&2 || true
    echo "----- tigerbeetle log -----" >&2
    docker logs "$TB_CONTAINER" >&2 2>&1 || true
  fi
  if [[ -n "$BACKEND_PID" ]] && kill -0 "$BACKEND_PID" 2>/dev/null; then
    kill "$BACKEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
  fi
  # `cargo run` spawns the binary as a child, which outlives a kill of cargo
  pkill -f 'target/debug/backend' 2>/dev/null || true
  docker rm -f "$PG_CONTAINER" "$TB_CONTAINER" >/dev/null 2>&1 || true
  rm -f "$SERVER_LOG"
}
trap cleanup EXIT

# waits until a TCP port accepts a connection, or fails after $3 seconds
wait_for_port() {
  local host=$1 port=$2 tries=${3:-30}
  for _ in $(seq 1 "$tries"); do
    if (exec 3<>"/dev/tcp/${host}/${port}") 2>/dev/null; then
      exec 3>&-
      return 0
    fi
    sleep 1
  done
  return 1
}

# --- postgres ----------------------------------------------------------------
echo "==> starting postgres ($PG_CONTAINER on :$DB_PORT)"
docker rm -f "$PG_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$PG_CONTAINER" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=backend \
  -p "${DB_PORT}:5432" \
  postgres:16 >/dev/null

echo "==> waiting for postgres"
for _ in $(seq 1 30); do
  docker exec "$PG_CONTAINER" pg_isready -U postgres >/dev/null 2>&1 && break
  sleep 1
done

# --- tigerbeetle (single replica, development mode) -------------------------
# --development drops the O_DIRECT / fsync requirements (CI filesystems often
# can't do direct IO) and shrinks the batch size, so one node fits comfortably
# in a CI runner. seccomp=unconfined + IPC_LOCK are required for io_uring and
# mlock, same as the tigerbeetle services in compose.yaml.
echo "==> starting tigerbeetle ($TB_CONTAINER on :$TB_PORT, image $TB_IMAGE)"
docker rm -f "$TB_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$TB_CONTAINER" \
  --security-opt seccomp=unconfined \
  --cap-add IPC_LOCK \
  -p "${TB_PORT}:3000" \
  --entrypoint sh \
  "$TB_IMAGE" -c '
    set -e
    FILE=/tmp/0.tigerbeetle
    if [ ! -f "$FILE" ]; then
      tigerbeetle format --cluster='"$TB_CLUSTER_ID"' --replica=0 --replica-count=1 --development "$FILE"
    fi
    exec tigerbeetle start --addresses=0.0.0.0:3000 --development "$FILE"
  ' >/dev/null

echo "==> waiting for tigerbeetle"
if ! wait_for_port 127.0.0.1 "$TB_PORT" 30; then
  echo "tigerbeetle did not open :$TB_PORT in time" >&2
  exit 1
fi

# --- backend ---------------------------------------------------------------
echo "==> building backend"
DATABASE_URL="$DATABASE_URL" cargo build --locked

echo "==> starting backend (logs: $SERVER_LOG)"
DATABASE_URL="$DATABASE_URL" nohup cargo run --locked >"$SERVER_LOG" 2>&1 &
BACKEND_PID=$!

echo "==> waiting for backend"
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
  exit 1
fi

# --- tests ---------------------------------------------------------------
echo "==> running integration tests"
cargo test --locked --test integration_test
