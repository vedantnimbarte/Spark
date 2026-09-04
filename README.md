<img src="logo.svg" alt="Spark" width="280">

A self-hosted PaaS: push to Git, get a built, routed application on your own Kubernetes cluster.

- `api/` — Rust control plane (axum, sqlx, kube-rs)
- `web/` — Next.js dashboard
- `deploy/` — cluster manifests (Traefik, cert-manager issuers, in-cluster registry, RBAC)

Spark can run on the cluster it manages. See [Running Spark on the cluster](#running-spark-on-the-cluster).

## Running it

Requires Docker Desktop with **Kubernetes enabled** (Settings → Kubernetes), Rust, and Node.

```bash
# 1. Postgres
docker compose up -d

# 2. Cluster: Traefik, cert-manager issuers, in-cluster registry, RBAC
kubectl apply -k deploy/base
kubectl -n spark-system rollout status deploy/traefik
kubectl -n spark-system rollout status deploy/registry

# 3. cert-manager, for automatic TLS
bash deploy/setup-cert-manager.sh

# 4. Let the node's containerd pull from the in-cluster registry.
#    Re-run this after "Reset Kubernetes Cluster" in Docker Desktop.
bash deploy/setup-node-registry.sh

# 5. Control plane (migrations run at startup)
cd api && cp .env.example .env && cargo run

# 6. Dashboard, in a second terminal
cd web && pnpm install && pnpm dev
```

Open <http://localhost:3000>. The first account you create becomes the administrator; after that,
signup requires an existing session.

Applications are published at `<name>.localhost` over both HTTP and HTTPS — Traefik runs as a
LoadBalancer service, which Docker Desktop publishes on ports 80 and 443.

## TLS

cert-manager issues a certificate per application, driven entirely by the Ingress annotation and
`tls` block that Spark generates. `Domain.ssl_status` is read back from the Certificate's own Ready
condition, so it reflects what the cluster actually holds rather than what was intended.

Which issuer is used is one config value, `CLUSTER_ISSUER`:

| Issuer | Use |
|---|---|
| `spark-selfsigned` | Local development. No ACME challenge, so it works with no public DNS. Browsers show an interstitial. |
| `spark-letsencrypt-staging` | Getting DNS and firewall right, without burning rate limits. |
| `spark-letsencrypt` | Real certificates. Needs the domain to resolve to the cluster and port 80 reachable from the internet. |

It is one issuer per deployment, not per host, because cert-manager's annotation applies to the whole
Ingress and Let's Encrypt cannot sign a `.localhost` name. Before using either ACME issuer, set a
contact address in `deploy/base/issuers.yaml` — ACME registration requires one.

Both entrypoints stay open; there is no forced HTTP→HTTPS redirect. Add
`--entrypoints.web.http.redirections.entryPoint.to=websecure` to the Traefik args if you want one.

Set `COOKIE_SECURE=true` wherever the dashboard is served over HTTPS. It must stay `false` for plain
HTTP, because a browser silently discards a `Secure` cookie sent over HTTP.

## Running Spark on the cluster

```bash
# Builds both images and loads them onto the node (see the script for why it
# does not push through the registry).
bash deploy/build-images.sh

kubectl apply -k deploy/control-plane
kubectl -n spark-system rollout status deploy/spark-api
```

Then open <https://spark.localhost>. This brings its own Postgres as a StatefulSet with a PVC. To use
a database you already run, drop `postgres.yaml` and the `secretGenerator` from
`deploy/control-plane/kustomization.yaml` and supply your own `spark-db` Secret with a `DATABASE_URL`
key.

**Change `spark-change-me` in that generator before running this anywhere real.**

The API runs as the `spark-controller` ServiceAccount, so it holds the restricted permission set in
`deploy/base/control-plane-rbac.yaml` rather than a developer's kubeconfig.

## How a deployment works

1. A push webhook, or the Deploy button, records a `deployment` row and enqueues a job.
2. The worker resolves the branch to a commit SHA over git's smart HTTP protocol, so the build is
   pinned and reproducible.
3. A rootless BuildKit Job builds the repository's Dockerfile and pushes to the registry, with its
   log streamed line by line into Postgres.
4. The Deployment, Service, Ingress, Certificate and NetworkPolicy are applied, and the deployment is
   marked `deployed` only once a pod passes its readiness probe.

Work lives in a Postgres queue (`FOR UPDATE SKIP LOCKED`), so a deployment survives the control
plane restarting and is retried if the cluster is briefly unreachable.

## Checks

```bash
cd api && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd web && npx tsc --noEmit && pnpm build
```

The Rust tests cover the parts where a silent mistake produces a broken cluster object rather than a
compile error: manifest generation, webhook signature verification, the git ref parser, and input
validation. They need no cluster. `cargo test` and `cargo build` do need Postgres running, because
sqlx checks queries against the live schema at compile time — or set `SQLX_OFFLINE=true` to use the
committed `.sqlx` cache instead, which is what the Docker build does.

Run `cargo sqlx prepare` after changing a query or the schema, or the image build will fail.

## Known limits

- **NetworkPolicies are not enforced on Docker Desktop.** Its CNI is kindnet, which accepts policy
  objects and ignores them. The policies are correct and will take effect on a cluster running
  Calico or Cilium, but the control plane / data plane boundary is not actually enforced locally.
- **Let's Encrypt issuance is configured but unproven.** It cannot be exercised on Docker Desktop,
  which has no public DNS and no inbound port 80. Only the self-signed path has been run end to end.
- **Dockerfile builds only.** Nixpacks and Buildpacks are not wired up.
- **The build pod uses `hostNetwork`** so that `localhost:30500` means the same thing to the builder
  and the kubelet. Fine for a single-tenant homelab; revisit before running untrusted code.
- **Control-plane images are loaded onto the node, not pulled.** `imagePullPolicy: IfNotPresent`
  with a `latest` tag means a rebuild needs the pod restarted. On a real cluster, push the tags to a
  registry the nodes can reach and set the policy back to `Always`.
- **Registry contents do not survive a cluster reset** unless the PVC does. Set `REGISTRY_URL` and
  `REGISTRY_INSECURE=false` to push to an external registry instead.
- **No CI pipeline.** `cargo clippy -- -D warnings` and the test suite are run by hand.
