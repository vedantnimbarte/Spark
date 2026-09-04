#!/usr/bin/env bash
# Builds the control-plane images and loads them onto the Docker Desktop node.
#
# They are NOT pushed through the in-cluster registry. That registry is a
# NodePort, which Docker Desktop does not publish to the host, and a
# port-forward does not help either: the Docker daemon runs inside its own VM,
# so 127.0.0.1 there is not the Windows host the forward listens on.
#
# Instead the images are imported straight into the node's containerd, which is
# what `kind load docker-image` does. The manifests use imagePullPolicy:
# IfNotPresent so the kubelet uses what is already there.
#
# On a real cluster, push these tags to a registry the nodes can reach and set
# imagePullPolicy back to Always.
set -euo pipefail

cd "$(dirname "$0")/.."

REGISTRY="${REGISTRY:-localhost:30500}"
TAG="${TAG:-latest}"
NODE="${NODE:-desktop-control-plane}"

echo "==> regenerating the sqlx offline cache"
# The image build has no database, so queries are checked against this instead.
(cd api && cargo sqlx prepare)

for component in api web; do
  image="${REGISTRY}/spark/${component}:${TAG}"

  echo "==> building ${image}"
  # Attestation manifests turn the export into a multi-manifest index that
  # `ctr images import` will not accept as a single runnable image.
  docker build --provenance=false -t "${image}" "./${component}"

  echo "==> loading ${image} onto ${NODE}"
  docker save "${image}" | docker exec -i "${NODE}" ctr -n k8s.io images import -
done

echo "==> done. Deploy with: kubectl apply -k deploy/control-plane"
