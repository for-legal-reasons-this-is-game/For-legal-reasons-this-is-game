#!/bin/sh
set -eu
cd "$(dirname "$0")"
minikube start --cpus=6 -m=10g

# minikube's docker driver doesn't hard-cap CPU, so enforce it
docker update --cpus=6 minikube
sh backend/scripts/build-image.sh
kubectl apply -f k8s/ -R
kubectl apply -f backend/k8s/ -R
kubectl apply -f tigerbeetle/ -R
