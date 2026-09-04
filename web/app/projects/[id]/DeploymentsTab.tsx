"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError, shortSha, timeAgo, type Deployment } from "@/lib/api";
import { Button, Card, Empty, ErrorText, StatusBadge } from "@/components/ui";
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
  const queryClient = useQueryClient();

  const rollback = useMutation({
    mutationFn: (deploymentId: string) => api.rollback(deploymentId),
    onSuccess: (deployment) => {
      setSelected(deployment.id);
      return queryClient.invalidateQueries({ queryKey: ["deployments", appId] });
    },
  });

  const activeDeployment = deployments?.find((d) => d.id === active);
  // Only a deployment that produced a working image can be returned to, and
  // returning to the one already running is a no-op.
  const canRollback =
    activeDeployment?.status === "deployed" &&
    activeDeployment.image_ref !== null &&
    activeDeployment.id !== deployments?.[0]?.id;

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
                <span>{d.rolled_back_from ? "rollback" : d.status}</span>
                <span>{timeAgo(d.created_at)}</span>
              </div>
            </button>
          </li>
        ))}
      </ul>

      <div className="min-w-0 space-y-3">
        <Card className="min-w-0 overflow-hidden">
          {active ? (
            <LogView key={active} appId={appId} deploymentId={active} />
          ) : (
            <p className="text-faint p-4 text-sm">Select a deployment.</p>
          )}
        </Card>

        {canRollback && activeDeployment && (
          <div className="flex flex-wrap items-center gap-3">
            <Button
              onClick={() => rollback.mutate(activeDeployment.id)}
              disabled={rollback.isPending}
            >
              {rollback.isPending
                ? "Rolling back..."
                : `Roll back to ${shortSha(activeDeployment.commit_sha)}`}
            </Button>
            <p className="text-faint text-xs">
              Redeploys this image without rebuilding.
            </p>
          </div>
        )}

        {rollback.error instanceof ApiError && (
          <ErrorText>{rollback.error.message}</ErrorText>
        )}
      </div>
    </div>
  );
}
