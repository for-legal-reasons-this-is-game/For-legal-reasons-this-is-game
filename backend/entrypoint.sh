#!/bin/sh
# =============================================================================
#  Wraps the real process in `infisical run`, so the backend starts with every
#  secret from the Infisical project injected as an environment variable.
#
#  The machine-identity credentials are produced by the infisical-setup
#  container and delivered on the shared volume mounted at /shared.
# =============================================================================
set -e

CREDS="/shared/infisical.env"

if [ ! -f "$CREDS" ]; then
  echo "entrypoint: $CREDS missing - infisical-setup has not run yet" >&2
  exit 1
fi

# shellcheck disable=SC1090
set -a
. "$CREDS"
set +a

# Exchange the universal-auth client id/secret for a short-lived token.
INFISICAL_TOKEN="$(infisical login \
  --method=universal-auth \
  --client-id="$INFISICAL_UNIVERSAL_AUTH_CLIENT_ID" \
  --client-secret="$INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET" \
  --domain="$INFISICAL_API_URL" \
  --plain --silent)"
export INFISICAL_TOKEN

exec infisical run \
  --projectId="$INFISICAL_PROJECT_ID" \
  --env="${INFISICAL_ENV:-prod}" \
  --domain="$INFISICAL_API_URL" \
  --path=/ \
  -- "$@"
