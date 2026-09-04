<img src="logo.svg" alt="Spark" width="280">

A self-hosted PaaS: push to Git, get a built, routed, TLS-terminated application on your own
Kubernetes cluster. No recurring cost, no vendor lock-in.

Spark runs on the cluster it manages — see [Running Spark on the cluster](#running-spark-on-the-cluster).

## What it does

- **GitOps deploys** — a push webhook (GitHub or GitLab, signature verified) or a Deploy button
  builds and ships the current branch.
- **Container builds** — rootless BuildKit runs each build as a Kubernetes Job, with a layer cache
  kept in the registry and the log streamed live to the dashboard.
- **Private repositories** — a deploy token per application, held apart from the app's own secrets.
- **Automatic TLS** — cert-manager issues a certificate per application; `ssl_status` is read back
  from the certificate rather than assumed.
- **Custom domains** — added or removed without a redeploy.
- **Environment variables** — stored only in Kubernetes, never in the database.
- **Rollback** — redeploy the exact image a previous deployment produced.
- **Resource limits and scaling** — CPU, memory and replica count per application.
- **Health and usage** — pod phase, restarts, and live CPU/memory in the dashboard.

## Layout

| Path | What |
|---|---|
| `api/` | Rust control plane — axum, sqlx, kube-rs |
| `web/` | Next.js dashboard — TypeScript, Tailwind, TanStack Query |
| `deploy/base/` | Traefik, cert-manager issuers, in-cluster registry, RBAC |
| `deploy/control-plane/` | Spark itself, plus its Postgres |
| `.github/workflows/` | CI |

The control plane is layered: `handlers/` (HTTP) → `services/` (domain logic) → `repos/` (SQL), with
`k8s/` holding the cluster client and pure manifest builders, and `queue/` the background worker.

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

# 4. Optional: live CPU and memory in the dashboard
bash deploy/setup-metrics-server.sh

# 5. Let the node's containerd pull from the in-cluster registry.
#    Re-run this after "Reset Kubernetes Cluster" in Docker Desktop.
bash deploy/setup-node-registry.sh

# 6. Control plane (migrations run at startup)
cd api && cp .env.example .env && cargo run

# 7. Dashboard, in a second terminal
cd web && pnpm install && pnpm dev
```

Open <http://localhost:3000>. The first account you create becomes the administrator; after that,
signup requires an existing session.

Applications are published at `<name>.localhost` over both HTTP and HTTPS — Traefik runs as a
LoadBalancer service, which Docker Desktop publishes on ports 80 and 443.

## How a deployment works

1. A push webhook, or the Deploy button, records a `deployment` row and enqueues a job.
2. The worker resolves the branch to a commit SHA over git's smart HTTP protocol, so the build is
   pinned and reproducible.
3. A rootless BuildKit Job builds the repository's Dockerfile and pushes to the registry, with its
   log streamed line by line into Postgres.
4. The Deployment, Service, Ingress, Certificate and NetworkPolicy are applied, and the deployment is
   marked `deployed` only once a pod passes its readiness probe.

Work lives in a Postgres queue (`SELECT … FOR UPDATE SKIP LOCKED`), so a deployment survives the
control plane restarting and is retried if the cluster is briefly unreachable. A lock older than 30
minutes is treated as abandoned and reclaimed.

Only one deployment per application runs at a time, enforced by a partial unique index. A second
request while one is in flight returns 409 rather than racing it — otherwise the slower build could
finish last and ship the older commit.

Each application gets its own namespace (`spark-app-<id>`), created when the application is created
rather than at first deploy, because environment variables can be set before anything is built.
Deleting an application deletes the namespace and everything in it.

## Environment variables

Values live **only** in the application's Kubernetes Secret. Postgres stores key names, so a database
dump leaks nothing. The dashboard lists names and reveals a value one at a time, on request, at the
cost of a cluster round-trip. Saving restarts the application, because `envFrom` values are injected
at container start and do not update in place.

The trade-off: a cluster reset loses the values, since Postgres never had them.

## Private repositories

Add a personal access token under an application's Settings. It is kept in a Kubernetes Secret of its
own (`app-git`), separate from the environment Secret injected into the running container — so an
application can never read its own deploy token.

The control plane uses it to resolve the branch, and BuildKit receives it as a build secret rather
than in the clone URL, keeping it out of the build log.

HTTPS tokens only; SSH deploy keys are not supported.

## Rolling back

Every deployment records the image it produced, so rolling back redeploys that exact image rather
than rebuilding the commit — a rebuild could pick up a moved base image and produce something
different. Pick a previous deployment and press Roll back; the build step is skipped entirely.

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

## Webhooks

Each application has its own webhook URL and secret, shown under Settings. GitHub payloads are
verified by HMAC-SHA256 over the raw body; GitLab by a constant-time token comparison. Pushes to a
branch other than the application's are acknowledged and ignored, as are tags, pings and branch
deletions.

The application id is part of the path (`/api/v1/webhooks/github/:app_id`) because a bare endpoint
gives no way to know which application a push belongs to or which secret to verify it against.

## Housekeeping

An hourly pass removes expired sessions, prunes deployments beyond the newest 20 per application, and
deletes the images those deployments pushed. The currently deployed one is never pruned, whatever its
age.

Login attempts are limited per email address — argon2 makes each guess expensive, but not impossible.

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
`deploy/base/control-plane-rbac.yaml` rather than a developer's kubeconfig. Running it in-cluster is
the only thing that actually exercises that permission set — two gaps in it were found exactly this
way, and neither was visible when running against a developer kubeconfig.

## Configuration

Read once at startup, from the environment or `api/.env`.

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | *required* | Postgres connection string |
| `BIND_ADDR` | `0.0.0.0:8080` | Listen address |
| `APP_BASE_DOMAIN` | `localhost` | Applications are published at `<name>.<domain>` |
| `REGISTRY_URL` | `localhost:30500` | Registry builds push to and the kubelet pulls from |
| `REGISTRY_INSECURE` | `true` | Allow plain HTTP to the registry |
| `CLUSTER_ISSUER` | `spark-selfsigned` | cert-manager issuer; empty disables TLS |
| `COOKIE_SECURE` | `false` | Set on the session cookie; requires HTTPS |
| `BUILD_CACHE` | `true` | Import/export a BuildKit layer cache in the registry |
| `CLUSTER_CIDRS` | `10.96.0.0/12,10.244.0.0/16` | Internal ranges user workloads are kept off |

## API

All under `/api/v1`, session-cookie authenticated except the webhooks, which are signature verified.

| Method | Path | |
|---|---|---|
| `POST` | `/auth/signup` | Open only while the instance has no users |
| `POST` | `/auth/login` · `/auth/logout` | |
| `GET` | `/auth/me` | |
| `GET` `POST` | `/apps` | List, create |
| `GET` `PATCH` `DELETE` | `/apps/{id}` | |
| `POST` | `/apps/{id}/deploy` | |
| `GET` | `/apps/{id}/deployments` | |
| `GET` | `/apps/{id}/health` | Live pod status and usage |
| `GET` | `/apps/{id}/webhook` | Webhook URL and secret |
| `GET` `PUT` | `/apps/{id}/env` | List key names, set values |
| `GET` `DELETE` | `/apps/{id}/env/{key}` | Reveal one value, remove one |
| `PUT` `DELETE` | `/apps/{id}/git-credentials` | Set or clear the deploy token |
| `GET` `POST` | `/apps/{id}/domains` | |
| `DELETE` | `/apps/{id}/domains/{domain_id}` | |
| `GET` | `/deployments/{id}` | |
| `GET` | `/deployments/{id}/logs` | Build log as server-sent events |
| `POST` | `/deployments/{id}/rollback` | |
| `POST` | `/webhooks/github/{id}` · `/webhooks/gitlab/{id}` | |
| `GET` | `/health` | Liveness, including a database round-trip |

## Checks

```bash
cd api && cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
cd web && npx tsc --noEmit && pnpm build
```

34 Rust tests cover the parts where a silent mistake produces a broken cluster object rather than a
compile error: manifest generation, webhook signature verification, the git ref parser, resource
quantity parsing, rate limiting, and input validation. They need no cluster.

`cargo test` and `cargo build` do need Postgres running, because sqlx checks queries against the live
schema at compile time — or set `SQLX_OFFLINE=true` to use the committed `.sqlx` cache instead, which
is what the Docker build and CI do.

**Run `cargo sqlx prepare` after changing a query or the schema**, or the image build and CI will
fail.

CI runs all of the above on push and pull request.

## Known limits

- **NetworkPolicies are not enforced on Docker Desktop.** Its CNI is kindnet, which accepts policy
  objects and ignores them. The policies are correct and will take effect on a cluster running Calico
  or Cilium, but the control plane / data plane boundary is not actually enforced locally.
- **Let's Encrypt issuance is configured but unproven.** It cannot be exercised on Docker Desktop,
  which has no public DNS and no inbound port 80. Only the self-signed path has been run end to end.
- **The private-repository path is unproven end to end.** Every piece is in place and tested in
  isolation, but no authenticated clone against a real private repository has been run.
- **Dockerfile builds only.** Nixpacks and Buildpacks are not wired up.
- **The build pod uses `hostNetwork`** so that `localhost:30500` means the same thing to the builder
  and the kubelet. Fine for a single-tenant homelab; revisit before running untrusted code.
- **Control-plane images are loaded onto the node, not pulled.** `imagePullPolicy: IfNotPresent` with
  a `latest` tag means a rebuild needs the pod restarted. On a real cluster, push the tags to a
  registry the nodes can reach and set the policy back to `Always`.
- **Login limiting is per-process and in-memory**, so it resets when the control plane restarts and
  does not span replicas. That matches a single-replica control plane; move it to Postgres before
  scaling out.
- **Registry disk is reclaimed in two steps.** Retention deletes old deployment manifests through the
  registry API, but the blobs are only freed when the registry's own garbage collection runs.
- **Registry contents do not survive a cluster reset** unless the PVC does. Set `REGISTRY_URL` and
  `REGISTRY_INSECURE=false` to push to an external registry instead.
- **Rolling deploys are Kubernetes' default**, and there is no blue/green or canary support.
