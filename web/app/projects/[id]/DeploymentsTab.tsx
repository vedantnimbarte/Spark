"use client";

import { useState } from "react";
import { shortSha, timeAgo, type Deployment } from "@/lib/api";
import { Card, Empty, StatusBadge } from "@/components/ui";
import { LogView } from "./LogView";

export function DeploymentsTab({
  appId,
  deployments,
}: {
  appId: string;
  deployments: Deployment[] | undefined;
}) {
  // Opens on the newest deployment so a running build is visible immediately.
  const [selected, setSelected] = useState<string | null>(null);
  const active = selected ?? deployments?.[0]?.id ?? null;

  if (!deployments) return <p className="text-faint text-sm">Loading…</p>;

  if (deployments.length === 0) {
    return (
      <Empty
        title="No deployments yet"
        hint="Press Deploy to build the current branch."
      />
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[18rem_1fr]">
      <ul className="space-y-1.5">
        {deployments.map((d) => (
          <li key={d.id}>
            <button
              onClick={() => setSelected(d.id)}
              className={`w-full rounded-md border px-3 py-2.5 text-left transition-colors ${
                d.id === active
                  ? "border-border-strong bg-surface"
                  : "border-border hover:border-border-strong"
              }`}
            >
              <div className="flex items-center justify-between gap-2">
                <span className="font-mono text-xs">
                  {shortSha(d.commit_sha)}
                </span>
                <StatusBadge status={d.status} showLabel={false} />
              </div>
              <div className="text-faint mt-1.5 flex items-center justify-between text-xs">
                <span>{d.status}</span>
                <span>{timeAgo(d.created_at)}</span>
              </div>
            </button>
          </li>
        ))}
      </ul>

      <Card className="min-w-0 overflow-hidden">
        {active ? (
          <LogView key={active} appId={appId} deploymentId={active} />
        ) : (
          <p className="text-faint p-4 text-sm">Select a deployment.</p>
        )}
      </Card>
    </div>
  );
}
