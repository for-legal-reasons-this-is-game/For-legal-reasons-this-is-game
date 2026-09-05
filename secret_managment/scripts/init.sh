#!/usr/bin/env bash
# =============================================================================
#  Zero-touch Infisical provisioner.
#
#  Runs on every `docker compose up`, but is a no-op after the first success
#  (guarded by the marker file on the shared volume).
#
#  Steps:
#    1. wait for the Infisical API
#    2. bootstrap the instance          -> instance-admin machine-identity token
#    3. create the project
#    4. push every key named in $INFISICAL_APP_SECRETS into <ENV>
#    5. create a scoped machine identity (universal auth) for the app services
#    6. grant that identity access to the project
#    7. write the identity credentials to the shared volume
# =============================================================================
set -euo pipefail

DOMAIN="${INFISICAL_API_URL:-http://infisical-backend:8080}"
SHARED_DIR="/shared"
MARKER="${SHARED_DIR}/infisical.env"

PROJECT_NAME="${INFISICAL_PROJECT_NAME:-transcendence}"
PROJECT_SLUG="${INFISICAL_PROJECT_SLUG:-transcendence}"
ENV_SLUG="${INFISICAL_ENV:-prod}"

log() { echo "[setup] $*"; }
die() { echo "[setup] ERROR: $*" >&2; exit 1; }

# --- idempotency -------------------------------------------------------------
if [ -f "$MARKER" ]; then
  log "already provisioned ($MARKER present) - nothing to do"
  chmod 0644 "$MARKER" 2>/dev/null || true
  exit 0
fi

# --- 1. wait for the API ----------------------------------------------------
log "waiting for Infisical at $DOMAIN ..."
for _ in $(seq 1 60); do
  curl -sf "$DOMAIN/api/status" >/dev/null 2>&1 && break
  sleep 2
done
curl -sf "$DOMAIN/api/status" >/dev/null 2>&1 || die "Infisical never became ready"
log "Infisical is up"

# --- 2. bootstrap ---------------------------------------------------------
log "bootstrapping instance (admin: ${INFISICAL_ADMIN_EMAIL})"
BOOT_ERR="$(mktemp)"
if ! BOOT="$(infisical bootstrap \
  --domain="$DOMAIN" \
  --email="$INFISICAL_ADMIN_EMAIL" \
  --password="$INFISICAL_ADMIN_PASSWORD" \
  --organization="$INFISICAL_ORG_NAME" \
  --output json --silent 2>"$BOOT_ERR")"; then
  die "bootstrap failed:
$(cat "$BOOT_ERR")

If the Infisical DB already has an admin but the shared volume was wiped,
the instance is in an inconsistent state -> run 'make reset'."
fi
rm -f "$BOOT_ERR"

TOKEN="$(echo "$BOOT" | jq -r '.identity.credentials.token')"
ORG_ID="$(echo "$BOOT" | jq -r '.organization.id')"
[ -n "$TOKEN" ] && [ "$TOKEN" != "null" ] || die "no token in bootstrap response:
$BOOT"
export INFISICAL_TOKEN="$TOKEN"

api() { # api METHOD PATH [JSON_BODY]
  local method="$1" path="$2" body="${3:-}"
  curl -sf -X "$method" "${DOMAIN}${path}" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/json" \
    ${body:+-d "$body"}
}

# --- 3. create project --------------------------------------------------
log "creating project '$PROJECT_NAME'"
PROJ="$(api POST /api/v1/projects \
  "{\"projectName\":\"$PROJECT_NAME\",\"slug\":\"$PROJECT_SLUG\",\"type\":\"secret-manager\"}")" \
  || die "project creation request failed"
PROJECT_ID="$(echo "$PROJ" | jq -r '.project.id // .project._id')"
[ -n "$PROJECT_ID" ] && [ "$PROJECT_ID" != "null" ] || die "no project id in response:
$PROJ"
log "project id: $PROJECT_ID"

# --- 4. push the application secrets -----------------------------------
#  $INFISICAL_APP_SECRETS is a space-separated list of variable names, coming
#  from .env via the container's env_file. The values are already in this
#  script's environment for the same reason.
raw_list="${INFISICAL_APP_SECRETS:-}"
raw_list="${raw_list%\"}"; raw_list="${raw_list#\"}"     # tolerate quoted value
read -ra APP_KEYS <<< "$raw_list"
[ "${#APP_KEYS[@]}" -gt 0 ] || die "INFISICAL_APP_SECRETS is empty - nothing to push"

log "pushing ${#APP_KEYS[@]} secrets into env '$ENV_SLUG': ${APP_KEYS[*]}"
set_args=()
for k in "${APP_KEYS[@]}"; do
  v="${!k:-}"
  [ -n "$v" ] || { log "  WARN: \$$k is unset/empty - skipping"; continue; }
  set_args+=("$k=$v")
done
[ "${#set_args[@]}" -gt 0 ] || die "none of the listed keys had a value"
infisical secrets set "${set_args[@]}" \
  --projectId="$PROJECT_ID" --env="$ENV_SLUG" --domain="$DOMAIN"

# --- 5. scoped machine identity for the app services -----------------
log "creating machine identity 'microservices'"
IDENT="$(api POST /api/v1/identities \
  "{\"name\":\"microservices\",\"organizationId\":\"$ORG_ID\",\"role\":\"no-access\"}")" \
  || die "identity creation failed"
IDENTITY_ID="$(echo "$IDENT" | jq -r '.identity.id')"
[ -n "$IDENTITY_ID" ] && [ "$IDENTITY_ID" != "null" ] || die "no identity id:
$IDENT"

UA="$(api POST "/api/v1/auth/universal-auth/identities/$IDENTITY_ID" \
  '{"clientSecretTrustedIps":[{"ipAddress":"0.0.0.0/0"}],"accessTokenTrustedIps":[{"ipAddress":"0.0.0.0/0"}],"accessTokenTTL":0,"accessTokenMaxTTL":0,"accessTokenNumUsesLimit":0}')" \
  || die "attaching universal-auth failed"
CLIENT_ID="$(echo "$UA" | jq -r '.identityUniversalAuth.clientId')"

CS="$(api POST "/api/v1/auth/universal-auth/identities/$IDENTITY_ID/client-secrets" \
  '{"description":"docker-compose","numUsesLimit":0,"ttl":0}')" \
  || die "creating client secret failed"
CLIENT_SECRET="$(echo "$CS" | jq -r '.clientSecret')"
[ -n "$CLIENT_SECRET" ] && [ "$CLIENT_SECRET" != "null" ] || die "no client secret:
$CS"

# --- 6. grant the identity access to the project ---------------------
log "granting identity access to project"
api POST "/api/v2/workspace/$PROJECT_ID/identity-memberships/$IDENTITY_ID" \
  '{"role":"member"}' >/dev/null 2>&1 \
  || api POST "/api/v1/projects/$PROJECT_ID/identities/$IDENTITY_ID" \
       '{"role":"member"}' >/dev/null \
  || die "could not add identity to project (check API paths for your Infisical version)"

# --- 7. hand credentials to the app services -------------------------
log "writing consumer credentials to $MARKER"
cat > "$MARKER" <<EOF
INFISICAL_API_URL=$DOMAIN
INFISICAL_PROJECT_ID=$PROJECT_ID
INFISICAL_ENV=$ENV_SLUG
INFISICAL_UNIVERSAL_AUTH_CLIENT_ID=$CLIENT_ID
INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET=$CLIENT_SECRET
EOF
# app services run as non-root; the shared volume is only reachable from
# inside the compose network.
chmod 0644 "$MARKER"

log "provisioning complete"
