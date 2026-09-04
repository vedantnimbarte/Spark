"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import { api, ApiError, formatBytes, formatCpu } from "@/lib/api";
import { BranchIcon, ExternalIcon } from "@/components/icons";
import { Button, Card, StatusBadge } from "@/components/ui";
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
      <main className="px-8 py-10">
        <p className="text-muted text-sm">That project does not exist.</p>
        <Link href="/projects" className="text-accent mt-2 block text-sm">
          Back to projects
        </Link>
      </main>
    );
  }

  if (!app.data) {
    return <p className="text-faint px-8 py-10 text-sm">Loading…</p>;
  }

  const last = deployments.data?.[0];
  const url = `http://${app.data.name}.localhost`;
  const live = health.data?.ready ?? false;

  return (
    <main className="mx-auto max-w-5xl px-8 py-10">
      <Link
        href="/projects"
        className="text-faint hover:text-muted text-xs transition-colors"
      >
        ← Projects
      </Link>

      <header className="mt-3 flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <div className="flex items-center gap-3">
            <h1 className="truncate text-xl font-semibold tracking-tight">
              {app.data.name}
            </h1>
            {last && <StatusBadge status={last.status} />}
          </div>

          <div className="text-faint mt-2 flex flex-wrap items-center gap-4 text-xs">
            <span className="flex items-center gap-1.5">
              <BranchIcon className="size-3.5" />
              {app.data.git_branch}
            </span>
            <span className="truncate">{app.data.git_repo}</span>
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
            disabled={deploy.isPending}
          >
            {deploy.isPending ? "Queuing…" : "Deploy"}
          </Button>
        </div>
      </header>

      {/* Live health, straight from the cluster. */}
      <Card className="mt-6 flex flex-wrap items-center gap-x-8 gap-y-3 px-5 py-4">
        <Stat
          label="Status"
          value={
            <span className={live ? "text-success" : "text-muted"}>
              {live ? "Running" : (health.data?.pods[0]?.phase ?? "Not running")}
            </span>
          }
        />
        <Stat
          label="Replicas"
          value={`${health.data?.ready_replicas ?? 0}/${health.data?.replicas ?? 0}`}
        />
        <Stat label="Restarts" value={health.data?.restarts ?? 0} />
        <Stat
          label="CPU"
          value={
            health.data?.cpu_millicores == null
              ? "--"
              : formatCpu(health.data.cpu_millicores)
          }
        />
        <Stat
          label="Memory"
          value={
            health.data?.memory_bytes == null
              ? "--"
              : formatBytes(health.data.memory_bytes)
          }
        />
        <Stat label="Limits" value={`${app.data.cpu_limit} · ${app.data.memory_limit}`} />
        <Stat
          label="URL"
          value={
            <a href={url} target="_blank" rel="noreferrer" className="text-accent">
              {app.data.name}.localhost
            </a>
          }
        />
      </Card>

      <nav className="border-border mt-8 flex gap-1 border-b">
        {TABS.map((t) => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className={`-mb-px border-b px-3 py-2 text-sm transition-colors ${
              tab === t
                ? "border-fg text-fg"
                : "text-muted hover:text-fg border-transparent"
            }`}
          >
            {t}
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

function Stat({
  label,
  value,
}: {
  label: string;
  value: React.ReactNode;
}) {
  return (
    <div>
      <p className="text-faint text-xs">{label}</p>
      <p className="mt-1 text-sm">{value}</p>
    </div>
  );
}
