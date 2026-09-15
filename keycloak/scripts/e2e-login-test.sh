#!/usr/bin/env bash
# Drives the browser-based auth-code+PKCE login flow against the `e2e` realm's
# `frontend` client, then exchanges the resulting code for tokens.
# Requires `docker compose up -d` to already be running (see compose.yaml).
set -euo pipefail

KEYCLOAK_URL="${KEYCLOAK_URL:-http://localhost:8081}"
REALM="${REALM:-e2e}"
CLIENT_ID="${CLIENT_ID:-frontend}"
REDIRECT_URI="${REDIRECT_URI:-http://localhost:3000/}"

CODE_VERIFIER=$(openssl rand -base64 48 | tr -d '=+/' | cut -c1-64)
CODE_CHALLENGE=$(printf '%s' "$CODE_VERIFIER" | openssl dgst -sha256 -binary | openssl base64 | tr -d '=' | tr '+/' '-_')

AUTH_URL="${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/auth?client_id=${CLIENT_ID}&redirect_uri=${REDIRECT_URI}&response_type=code&scope=openid&code_challenge=${CODE_CHALLENGE}&code_challenge_method=S256"

echo "==> open this URL in a browser and log in (e.g. e2e-user / e2e-password):"
echo "$AUTH_URL"
echo
echo "==> after login you'll land on a page that fails to load (nothing runs on ${REDIRECT_URI})."
echo "    copy the 'code' query parameter from that browser address bar."
echo
read -rp "paste code: " AUTH_CODE

echo "==> exchanging code for tokens"
RESPONSE=$(curl -sf -X POST "${KEYCLOAK_URL}/realms/${REALM}/protocol/openid-connect/token" \
  -d "grant_type=authorization_code" \
  -d "client_id=${CLIENT_ID}" \
  -d "code=${AUTH_CODE}" \
  -d "redirect_uri=${REDIRECT_URI}" \
  -d "code_verifier=${CODE_VERIFIER}")

if command -v jq >/dev/null 2>&1; then
  echo "$RESPONSE" | jq .
else
  echo "$RESPONSE"
fi
