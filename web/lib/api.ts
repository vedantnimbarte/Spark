/**
 * Typed wrapper over the control plane API.
 *
 * Requests go to a same-origin `/api` path that Next rewrites to the Rust
 * service, so the session cookie travels without any CORS handling.
 */

export interface User {
  id: string;
  email: string;
  created_at: string;
}

export interface Application {
  id: string;
  owner_id: string;
  name: string;
  git_repo: string;
  git_branch: string;
  build_type: string;
  dockerfile_path: string;
  container_port: number;
  cpu_limit: string;
  memory_limit: string;
  replicas: number;
  git_credentials_set: boolean;
  created_at: string;
}

export type DeploymentStatus =
  | "pending"
  | "building"
  | "deploying"
  | "deployed"
  | "failed";

export interface Deployment {
  id: string;
  app_id: string;
  commit_sha: string;
  status: DeploymentStatus;
  image_ref: string | null;
  /** Set when this deployment reused an earlier deployment's image. */
  rolled_back_from: string | null;
  started_at: string | null;
  finished_at: string | null;
  created_at: string;
}

export interface Domain {
  id: string;
  app_id: string;
  domain_name: string;
  ssl_status: string;
  created_at: string;
}

export interface WebhookConfig {
  github_url: string;
  gitlab_url: string;
  secret: string;
}

export interface AppHealth {
  ready: boolean;
  replicas: number;
  ready_replicas: number;
  restarts: number;
  /** null when metrics-server is not installed, which is not the same as 0. */
  cpu_millicores: number | null;
  memory_bytes: number | null;
  pods: PodStatus[];
}

export interface PodStatus {
  name: string;
  phase: string;
  ready: boolean;
  restarts: number;
  message: string | null;
}

/** An API error carrying the status, so callers can tell 401 from 500. */
export class ApiError extends Error {
  readonly status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`/api/v1${path}`, {
    ...init,
    // Sends and accepts the session cookie.
    credentials: "same-origin",
    headers: {
      ...(init?.body ? { "content-type": "application/json" } : {}),
      ...init?.headers,
    },
  });

  if (!response.ok) {
    // The API returns {"error": "..."}; fall back to the status text when the
    // body is not JSON (a proxy error, say).
    let message = response.statusText;
    try {
      const body: unknown = await response.json();
      if (
        typeof body === "object" &&
        body !== null &&
        "error" in body &&
        typeof body.error === "string"
      ) {
        message = body.error;
      }
    } catch {
      // Keep the status text.
    }
    throw new ApiError(response.status, message);
  }

  if (response.status === 204) {
    return undefined as T;
  }
  return (await response.json()) as T;
}

export const api = {
  signup: (email: string, password: string) =>
    request<User>("/auth/signup", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),

  login: (email: string, password: string) =>
    request<User>("/auth/login", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),

  logout: () => request<void>("/auth/logout", { method: "POST" }),

  me: () => request<User>("/auth/me"),

  listApps: () => request<Application[]>("/apps"),

  getApp: (id: string) => request<Application>(`/apps/${id}`),

  createApp: (input: {
    name: string;
    git_repo: string;
    git_branch?: string;
    dockerfile_path?: string;
    container_port?: number;
    cpu_limit?: string;
    memory_limit?: string;
  }) =>
    request<Application>("/apps", {
      method: "POST",
      body: JSON.stringify(input),
    }),

  updateApp: (id: string, patch: Partial<Application>) =>
    request<Application>(`/apps/${id}`, {
      method: "PATCH",
      body: JSON.stringify(patch),
    }),

  deleteApp: (id: string) => request<void>(`/apps/${id}`, { method: "DELETE" }),

  deploy: (id: string) =>
    request<Deployment>(`/apps/${id}/deploy`, { method: "POST" }),

  listDeployments: (id: string) =>
    request<Deployment[]>(`/apps/${id}/deployments`),

  getDeployment: (id: string) => request<Deployment>(`/deployments/${id}`),

  rollback: (deploymentId: string) =>
    request<Deployment>(`/deployments/${deploymentId}/rollback`, {
      method: "POST",
    }),

  setGitCredentials: (id: string, token: string) =>
    request<void>(`/apps/${id}/git-credentials`, {
      method: "PUT",
      body: JSON.stringify({ token }),
    }),

  clearGitCredentials: (id: string) =>
    request<void>(`/apps/${id}/git-credentials`, { method: "DELETE" }),

  listEnv: (id: string) => request<{ keys: string[] }>(`/apps/${id}/env`),

  setEnv: (id: string, vars: Record<string, string>) =>
    request<{ keys: string[] }>(`/apps/${id}/env`, {
      method: "PUT",
      body: JSON.stringify(vars),
    }),

  revealEnv: (id: string, key: string) =>
    request<{ key: string; value: string }>(
      `/apps/${id}/env/${encodeURIComponent(key)}`,
    ),

  deleteEnv: (id: string, key: string) =>
    request<void>(`/apps/${id}/env/${encodeURIComponent(key)}`, {
      method: "DELETE",
    }),

  listDomains: (id: string) => request<Domain[]>(`/apps/${id}/domains`),

  addDomain: (id: string, domain_name: string) =>
    request<Domain>(`/apps/${id}/domains`, {
      method: "POST",
      body: JSON.stringify({ domain_name }),
    }),

  deleteDomain: (id: string, domainId: string) =>
    request<void>(`/apps/${id}/domains/${domainId}`, { method: "DELETE" }),

  webhook: (id: string) => request<WebhookConfig>(`/apps/${id}/webhook`),

  health: (id: string) => request<AppHealth>(`/apps/${id}/health`),
};

/** Short relative time, e.g. "4m ago". */
export function timeAgo(iso: string): string {
  const seconds = Math.max(0, (Date.now() - new Date(iso).getTime()) / 1000);

  const units: [number, string][] = [
    [60, "s"],
    [3600, "m"],
    [86400, "h"],
    [2592000, "d"],
  ];

  if (seconds < 60) return `${Math.floor(seconds)}s ago`;
  for (let i = 1; i < units.length; i += 1) {
    const unit = units[i];
    const previous = units[i - 1];
    if (!unit || !previous) break;
    if (seconds < unit[0]) {
      return `${Math.floor(seconds / previous[0])}${unit[1]} ago`;
    }
  }
  return `${Math.floor(seconds / 2592000)}mo ago`;
}

/** Bytes as a compact binary size, e.g. "48 MiB". */
export function formatBytes(bytes: number): string {
  const units = ["B", "KiB", "MiB", "GiB", "TiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value < 10 && unit > 0 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

/** Millicores as cores when large enough to read that way. */
export function formatCpu(millicores: number): string {
  return millicores >= 1000
    ? `${(millicores / 1000).toFixed(2)} cores`
    : `${millicores}m`;
}

export function shortSha(sha: string): string {
  // A branch name was recorded when the commit could not be resolved; showing
  // the first seven characters of that would be meaningless.
  return /^[0-9a-f]{40}$/.test(sha) ? sha.slice(0, 7) : sha;
}
