#!/usr/bin/env bash
# Installs cert-manager and waits for it to be ready.
#
# Applied from the upstream release manifest rather than vendored, because the
# file is ~1.5MB of mostly CRDs and pinning the version here is enough to keep
# the install reproducible.
set -euo pipefail

VERSION="${CERT_MANAGER_VERSION:-v1.21.1}"

echo "installing cert-manager ${VERSION}"
kubectl apply -f "https://github.com/cert-manager/cert-manager/releases/download/${VERSION}/cert-manager.yaml"

# The webhook must be serving before any Issuer or Certificate is accepted.
for deployment in cert-manager cert-manager-webhook cert-manager-cainjector; do
  kubectl -n cert-manager rollout status "deploy/${deployment}" --timeout=180s
done

echo "cert-manager ${VERSION} ready"
