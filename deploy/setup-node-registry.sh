#!/usr/bin/env bash
# Teach the Docker Desktop node's containerd to pull from the in-cluster
# registry directly.
#
# Docker Desktop installs /etc/containerd/certs.d/_default/hosts.toml, which
# routes EVERY registry through its own mirror. Without a host-specific entry
# our localhost:30500 pulls are sent to that mirror, which does not have the
# image, and the kubelet reports "short read: expected N bytes but got 0".
#
# containerd re-reads certs.d per pull, so no restart is needed. This lives on
# the node, so re-run it after "Reset Kubernetes Cluster" in Docker Desktop.
set -euo pipefail

NODE="${NODE:-desktop-control-plane}"
REGISTRY="${REGISTRY:-localhost:30500}"

docker exec -i "$NODE" sh -s <<INNER
set -eu
mkdir -p "/etc/containerd/certs.d/${REGISTRY}"
cat > "/etc/containerd/certs.d/${REGISTRY}/hosts.toml" <<'TOML'
server = "http://${REGISTRY}"

[host."http://${REGISTRY}"]
  capabilities = ["pull", "resolve"]
  skip_verify = true
TOML
INNER

echo "configured ${REGISTRY} on node ${NODE}"
