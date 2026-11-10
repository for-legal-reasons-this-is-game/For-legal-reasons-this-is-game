#!/usr/bin/env bash
# Builds the backend image directly inside minikube (run from repo root).
set -eu
cd "$(dirname "$0")/../.."
minikube image build -f backend/Dockerfile -t backend:latest .
