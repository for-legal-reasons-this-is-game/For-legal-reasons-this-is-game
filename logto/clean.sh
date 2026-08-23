#!/bin/sh
echo "Stoping logto with database and clearing all info"
docker compose -p logto -f docker-compose.yaml down -v
