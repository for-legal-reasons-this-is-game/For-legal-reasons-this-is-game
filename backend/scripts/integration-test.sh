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

BACKEND_PORT="${BACKEND_PORT:-8000}"
SERVER_LOG="$(mktemp)"
BACKEND_PID=""

cleanup() {
  local code=$?
  if [[ $code -ne 0 ]]; then
    echo "==> FAILED (exit $code); dumping diagnostics" >&2
    echo "----- docker ps -a -----" >&2
    docker ps -a >&2 2>&1 || true
    echo "----- backend log -----" >&2
    cat "$SERVER_LOG" >&2 2>&1 || true
    echo "----- postgres log (tail) -----" >&2
    docker logs --tail 50 "$PG_CONTAINER" >&2 2>&1 || true
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

container_running() {
  [[ "$(docker inspect -f '{{.State.Running}}' "$1" 2>/dev/null)" == "true" ]]
}

# Raw HTTP/1.0 GET against the backend via bash's /dev/tcp (curl is not in the
# nix devshell and misbehaves there). Echoes the numeric status, or nothing on a
# refused connection.
http_status() {
  local path=$1 line
  exec 9<>"/dev/tcp/127.0.0.1/${BACKEND_PORT}" 2>/dev/null || return 1
  printf 'GET %s HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n' "$path" >&9 || { exec 9>&- 9<&-; return 1; }
  IFS=' ' read -r _ line _ <&9 || line=""
  exec 9>&- 9<&- 2>/dev/null || true
  printf '%s' "$line"
}

# Full raw response, for diagnostics.
http_dump() {
  local path=$1
  exec 9<>"/dev/tcp/127.0.0.1/${BACKEND_PORT}" 2>/dev/null || { echo "(connection refused)"; return; }
  printf 'GET %s HTTP/1.0\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n' "$path" >&9
  cat <&9 || true
  exec 9>&- 9<&- 2>/dev/null || true
}

# --- postgres --------------------------------------------------------------
echo "==> starting postgres ($PG_CONTAINER on :$DB_PORT)"
docker rm -f "$PG_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$PG_CONTAINER" \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=backend \
  -p "${DB_PORT}:5432" \
  postgres:16 >/dev/null

echo "==> waiting for postgres"
pg_ready=false
for _ in $(seq 1 60); do
  if ! container_running "$PG_CONTAINER"; then
    echo "postgres container exited" >&2
    exit 1
  fi
  # -d backend so we don't pass while initdb's bootstrap server is up but the
  # real one (with our DB) isn't listening on TCP yet
  if docker exec "$PG_CONTAINER" pg_isready -U postgres -d backend -q >/dev/null 2>&1; then
    pg_ready=true
    break
  fi
  sleep 1
done
[[ "$pg_ready" == true ]] || { echo "postgres never became ready" >&2; exit 1; }

# --- tigerbeetle (single replica, development mode) ------------------------
# --development drops the O_DIRECT / fsync requirements (CI filesystems often
# can't do direct IO) and shrinks the batch size, so one node fits comfortably
# in a CI runner. seccomp=unconfined + IPC_LOCK are required for io_uring and
# mlock, same as the tigerbeetle services in compose.yaml. The binary lives at
# /tigerbeetle in the (alpine) image and is NOT on $PATH.
echo "==> starting tigerbeetle ($TB_CONTAINER on :$TB_PORT, image $TB_IMAGE)"
docker rm -f "$TB_CONTAINER" >/dev/null 2>&1 || true
docker run -d --name "$TB_CONTAINER" \
  --security-opt seccomp=unconfined \
  --cap-add IPC_LOCK \
  -e TB_CLUSTER_ID="$TB_CLUSTER_ID" \
  -p "${TB_PORT}:3000" \
  --entrypoint /bin/sh \
  "$TB_IMAGE" -c '
    set -e
    FILE=/tmp/0.tigerbeetle
    [ -f "$FILE" ] || /tigerbeetle format --cluster="$TB_CLUSTER_ID" --replica=0 --replica-count=1 --development "$FILE"
    exec /tigerbeetle start --addresses=0.0.0.0:3000 --development "$FILE"
  ' >/dev/null

echo "==> waiting for tigerbeetle"
tb_ready=false
for _ in $(seq 1 60); do
  if ! container_running "$TB_CONTAINER"; then
    echo "tigerbeetle container exited" >&2
    exit 1
  fi
  if docker logs "$TB_CONTAINER" 2>&1 | grep -q 'listening on'; then
    tb_ready=true
    break
  fi
  sleep 1
done
[[ "$tb_ready" == true ]] || { echo "tigerbeetle never logged 'listening on'" >&2; exit 1; }

# --- backend --------------------------------------------------------------
echo "==> building backend"
DATABASE_URL="$DATABASE_URL" cargo build --locked

echo "==> starting backend (logs: $SERVER_LOG)"
DATABASE_URL="$DATABASE_URL" nohup cargo run --locked >"$SERVER_LOG" 2>&1 &
BACKEND_PID=$!

echo "==> waiting for backend"
ready=false
for _ in $(seq 1 60); do
  if ! kill -0 "$BACKEND_PID" 2>/dev/null; then
    echo "backend process exited early" >&2
    exit 1
  fi
  if [[ "$(http_status /api/v1/users || true)" == "200" ]]; then
    ready=true
    break
  fi
  sleep 1
done
if [[ "$ready" != "true" ]]; then
  echo "backend did not become ready in time; last response:" >&2
  http_dump /api/v1/users >&2
  exit 1
fi

# --- tests ---------------------------------------------------------------
echo "==> running integration tests"
cargo test --locked --test integration_test
