# =============================================================================
#  For legal reasons this is game  -  one-command stack
# =============================================================================
COMPOSE := docker compose

.DEFAULT_GOAL := up
.PHONY: up down re logs ps setup build reset clean fclean

compile_tradingengine:
	cd tradingengine && nix develop -c cargo build

compile_backend:
	cd backend && nix develop -c cargo build

compile_all: compile_backend compile_tradingengine
