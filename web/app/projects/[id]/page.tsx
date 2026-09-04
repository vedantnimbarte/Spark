"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import {
  api,
  ApiError,
  formatBytes,
  formatCpu,
  shortSha,
  timeAgo,
} from "@/lib/api";
import { BranchIcon, ExternalIcon } from "@/components/icons";
import {
  ActivityLine,
  Button,
  Detail,
  isInFlight,
  Panel,
  Skeleton,
  StatusPill,
} from "@/components/ui";
import { DeploymentsTab } from "./DeploymentsTab";
import { DomainsTab } from "./DomainsTab";
import { EnvTab } from "./EnvTab";
import { SettingsTab } from "./SettingsTab";

const TABS = ["Deployments", "Environment", "Domains", "Settings"] as const;
type Tab = (typeof TABS)[number];

export default function ProjectPage() {
  const params = useParams<{ id: string }>();
  const id = params.id;
  const [tab, setTab] = useState<Tab>("Deployments");
  const queryClient = useQueryClient();

  const app = useQuery({
    queryKey: ["app", id],
    queryFn: () => api.getApp(id),
  });

  const health = useQuery({
    queryKey: ["health", id],
    queryFn: () => api.health(id),
    refetchInterval: 5_000,
  });

  const deployments = useQuery({
    queryKey: ["deployments", id],
    queryFn: () => api.listDeployments(id),
    refetchInterval: 3_000,
  });

  const deploy = useMutation({
    mutationFn: () => api.deploy(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["deployments", id] }),
  });

  if (app.error instanceof ApiError && app.error.status === 404) {
    return (
      <main className="mx-auto max-w-5xl px-8 py-10">
        <h1 className="text-[22px] font-semibold tracking-tight">
          Project not found
        </h1>
        <p className="text-muted mt-2 text-[13px]">
          It may have been deleted, or the link may be wrong.
        </p>
        <Link
          href="/projects"
          className="text-accent mt-4 inline-block text-[13px]"
        >
          Back to projects
        </Link>
      </main>
    );
  }

  if (!app.data) {
    return (
      <main className="mx-auto max-w-5xl space-y-4 px-8 py-10">
        <Skeleton className="h-7 w-48" />
        <Skeleton className="h-20 w-full" />
      </main>
    );
  }

  const last = deployments.data?.[0];
  const url = `http://${app.data.name}.localhost`;
  const running = health.data?.ready ?? false;
  const deploying = last ? isInFlight(last.status) : false;
  const failedPods = health.data?.pods.filter((pod) => !pod.ready) ?? [];

  return (
    <main className="mx-auto max-w-5xl px-8 py-10">
      <Link
        href="/projects"
        className="text-faint hover:text-muted text-xs transition-colors"
      >
        Projects
      </Link>

      <header className="mt-3 flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex flex-wrap items-center gap-3">
            <h1 className="truncate text-[22px] leading-tight font-semibold tracking-tight">
              {app.data.name}
            </h1>
            {last && <StatusPill status={last.status} />}
          </div>

          <div className="text-faint mt-2 flex flex-wrap items-center gap-x-4 gap-y-1 text-xs">
            <span className="flex items-center gap-1.5">
              <BranchIcon className="size-3.5" />
              {app.data.git_branch}
            </span>
            {last && (
              <span className="font-mono">{shortSha(last.commit_sha)}</span>
            )}
            {last && <span>deployed {timeAgo(last.created_at)}</span>}
            <span className="max-w-full truncate">{app.data.git_repo}</span>
          </div>
        </div>

        <div className="flex items-center gap-2">
          <a href={url} target="_blank" rel="noreferrer">
            <Button>
              <ExternalIcon className="size-3.5" />
              Visit
            </Button>
          </a>
          <Button
            variant="primary"
            onClick={() => deploy.mutate()}
            disabled={deploy.isPending || deploying}
          >
            {deploy.isPending
              ? "Queuing…"
              : deploying
                ? "Deploy running"
                : "Deploy"}
          </Button>
        </div>
      </header>

      {deploy.error instanceof ApiError && (
        <p role="alert" className="text-danger mt-3 text-[13px]">
          {deploy.error.message}
        </p>
      )}

      {/* Live from the cluster, so it cannot go stale the way a cached copy
          in Postgres would. */}
      <Panel className="mt-6 overflow-hidden">
        {deploying ? <ActivityLine /> : <div className="h-px" aria-hidden />}

        <div className="grid grid-cols-2 gap-x-6 gap-y-4 px-5 py-4 sm:grid-cols-3 lg:grid-cols-5">
          <Detail label="Status">
            {health.isPending ? (
              <Skeleton className="h-4 w-16" />
            ) : (
              <span className={running ? "text-success" : "text-muted"}>
                {running
                  ? "Running"
                  : (health.data?.pods[0]?.phase ?? "Not running")}
              </span>
            )}
          </Detail>

          <Detail label="Replicas">
            <span className="tnum font-mono">
              {health.data?.ready_replicas ?? 0}/{health.data?.replicas ?? 0}
            </span>
          </Detail>

          <Detail label="Restarts">
            <span
              className={`tnum font-mono ${
                (health.data?.restarts ?? 0) > 0 ? "text-danger" : ""
              }`}
            >
              {health.data?.restarts ?? 0}
            </span>
          </Detail>

          <Detail label="CPU">
            <span className="tnum font-mono">
              {health.data?.cpu_millicores == null
                ? "--"
                : formatCpu(health.data.cpu_millicores)}
            </span>
            <span className="text-faint ml-1.5 text-xs">
              of {app.data.cpu_limit}
            </span>
          </Detail>

          <Detail label="Memory">
            <span className="tnum font-mono">
              {health.data?.memory_bytes == null
                ? "--"
                : formatBytes(health.data.memory_bytes)}
            </span>
            <span className="text-faint ml-1.5 text-xs">
              of {app.data.memory_limit}
            </span>
          </Detail>
        </div>

        <div className="border-line flex flex-wrap items-center justify-between gap-2 border-t px-5 py-2.5">
          <a
            href={url}
            target="_blank"
            rel="noreferrer"
            className="text-accent font-mono text-xs hover:underline"
          >
            {app.data.name}.localhost
          </a>
          {health.data?.cpu_millicores == null && (
            <p className="text-faint text-xs">
              Install metrics-server to see CPU and memory use.
            </p>
          )}
        </div>
      </Panel>

      {/* A pod that will not become ready is the thing worth interrupting for;
          the message from the cluster usually says exactly what is wrong. */}
      {failedPods.length > 0 && (
        <div className="border-danger/30 bg-danger/5 mt-3 rounded-lg border px-5 py-3.5">
          <p className="text-danger text-[13px] font-medium">
            {failedPods.length} pod{failedPods.length === 1 ? "" : "s"} not
            ready
          </p>
          <ul className="mt-2 space-y-1">
            {failedPods.map((pod) => (
              <li key={pod.name} className="text-muted font-mono text-xs">
                {pod.name} · {pod.phase}
                {pod.message ? ` — ${pod.message}` : ""}
              </li>
            ))}
          </ul>
        </div>
      )}

      <nav className="border-line mt-8 flex gap-1 border-b" aria-label="Project">
        {TABS.map((name) => (
          <button
            key={name}
            onClick={() => setTab(name)}
            aria-current={tab === name ? "page" : undefined}
            className={`-mb-px border-b-2 px-3 py-2 text-[13px] transition-colors ${
              tab === name
                ? "border-accent text-fg font-medium"
                : "text-muted hover:text-fg border-transparent"
            }`}
          >
            {name}
          </button>
        ))}
      </nav>

      <div className="py-6">
        {tab === "Deployments" && (
          <DeploymentsTab appId={id} deployments={deployments.data} />
        )}
        {tab === "Environment" && <EnvTab appId={id} />}
        {tab === "Domains" && <DomainsTab appId={id} appName={app.data.name} />}
        {tab === "Settings" && <SettingsTab app={app.data} />}
      </div>
    </main>
  );
}
