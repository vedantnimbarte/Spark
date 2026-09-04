"use client";

import { useQueries, useQuery } from "@tanstack/react-query";
import { api, type Deployment } from "@/lib/api";
import { Card } from "@/components/ui";

/** Counts across every project, derived from data already being fetched. */
export default function AnalyticsPage() {
  const apps = useQuery({ queryKey: ["apps"], queryFn: () => api.listApps() });

  const deployments = useQueries({
    queries: (apps.data ?? []).map((app) => ({
      queryKey: ["deployments", app.id],
      queryFn: () => api.listDeployments(app.id),
    })),
  });

  const all: Deployment[] = deployments.flatMap((q) => q.data ?? []);
  const succeeded = all.filter((d) => d.status === "deployed").length;
  const failed = all.filter((d) => d.status === "failed").length;
  const finished = succeeded + failed;

  return (
    <main className="mx-auto max-w-4xl px-8 py-10">
      <h1 className="text-xl font-semibold tracking-tight">Analytics</h1>

      <div className="mt-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        <Metric label="Projects" value={apps.data?.length ?? 0} />
        <Metric label="Deployments" value={all.length} />
        <Metric label="Succeeded" value={succeeded} tone="text-success" />
        <Metric label="Failed" value={failed} tone={failed > 0 ? "text-danger" : ""} />
      </div>

      <p className="text-faint mt-4 text-xs">
        {finished === 0
          ? "No finished deployments yet."
          : `${Math.round((succeeded / finished) * 100)}% of finished deployments succeeded.`}
      </p>
    </main>
  );
}

function Metric({
  label,
  value,
  tone = "",
}: {
  label: string;
  value: React.ReactNode;
  tone?: string;
}) {
  return (
    <Card className="p-4">
      <p className="text-faint text-xs">{label}</p>
      <p className={`mt-2 text-2xl font-semibold tracking-tight ${tone}`}>
        {value}
      </p>
    </Card>
  );
}
