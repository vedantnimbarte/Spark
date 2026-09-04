#!/usr/bin/env bash
# Installs metrics-server, which backs the CPU and memory figures in the
# dashboard. Optional: without it Spark reports usage as unavailable rather
# than failing.
#
# Docker Desktop's kubelet serves its metrics endpoint with a self-signed
# certificate, so verification has to be turned off for this cluster.
set -euo pipefail

VERSION="${METRICS_SERVER_VERSION:-v0.7.2}"

kubectl apply -f "https://github.com/kubernetes-sigs/metrics-server/releases/download/${VERSION}/components.yaml"

kubectl -n kube-system patch deployment metrics-server --type=json \
  -p='[{"op":"add","path":"/spec/template/spec/containers/0/args/-","value":"--kubelet-insecure-tls"}]'

kubectl -n kube-system rollout status deploy/metrics-server --timeout=180s
echo "metrics-server ${VERSION} ready"
