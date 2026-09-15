# =============================================================================
#  For legal reasons this is game  -  one-command stack
# =============================================================================
COMPOSE := docker compose

.DEFAULT_GOAL := up
.PHONY: up down re logs ps setup build reset clean fclean

## up      : ensure .env exists, then build & start everything
up: setup
	$(COMPOSE) up --build -d
	@echo ""
	@echo "  Infisical UI : http://localhost:$${INFISICAL_PORT:-8080}"
	@echo "  Backend API  : http://localhost:$${BACKEND_PORT:-8000}"
	@echo ""
	@echo "  Following the provisioner (Ctrl-C is safe once it says 'complete'):"
	-$(COMPOSE) logs -f infisical-setup

## setup   : create .env from the template if you didn't provide one
setup: .env
.env:
	cp .env.example .env
	@echo ">> created .env from .env.example (throwaway dev secrets - replace for real use)"

## down    : stop and remove containers (keeps volumes / secrets)
down:
	$(COMPOSE) down

## re      : down + up
re: down up

## logs    : tail logs for all services
logs:
	$(COMPOSE) logs -f

## ps      : show container status
ps:
	$(COMPOSE) ps

## build   : (re)build local images only
build:
	$(COMPOSE) build

## reset   : wipe ALL state (Infisical DB, secrets, app DB, TigerBeetle) so the
##           next `up` bootstraps from scratch. Keeps your .env.
reset: clean

## clean   : down + remove volumes
clean fclean:
	$(COMPOSE) down -v --remove-orphans
