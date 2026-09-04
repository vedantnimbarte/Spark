"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import {
  api,
  ApiError,
  formatDuration,
  shortSha,
  timeAgo,
  type Deployment,
} from "@/lib/api";
import {
  Button,
  Empty,
  ErrorText,
  Panel,
  StatusBadge,
} from "@/components/ui";
import { LogView } from "./LogView";

/** Seconds a deployment took, or null while it is still running. */
function duration(deployment: Deployment): number | null {
  if (!deployment.started_at || !deployment.finished_at) return null;
  return (
    (new Date(deployment.finished_at).getTime() -
      new Date(deployment.started_at).getTime()) /
    1000
  );
}

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
      return queryClient.invalidateQueries({
        queryKey: ["deployments", appId],
      });
    },
  });

  const activeDeployment = deployments?.find((d) => d.id === active);
  // Only a deployment that produced a working image can be returned to, and
  // returning to the one already running is a no-op.
  const canRollback =
    activeDeployment?.status === "deployed" &&
    activeDeployment.image_ref !== null &&
    activeDeployment.id !== deployments?.[0]?.id;

  if (!deployments) {
    return <p className="text-faint text-[13px]">Loading…</p>;
  }

  if (deployments.length === 0) {
    return (
      <Empty
        title="No deployments yet"
        hint="Press Deploy to build the current branch and run it on your cluster."
      />
    );
  }

  return (
    <div className="grid gap-4 lg:grid-cols-[19rem_1fr]">
      <ul className="max-h-[32rem] space-y-1.5 overflow-y-auto pr-1">
        {deployments.map((deployment) => {
          const seconds = duration(deployment);
          const isActive = deployment.id === active;
          return (
            <li key={deployment.id}>
              <button
                onClick={() => setSelected(deployment.id)}
                aria-current={isActive ? "true" : undefined}
                className={`w-full rounded-md border px-3 py-2.5 text-left transition-colors ${
                  isActive
                    ? "border-line-strong bg-raised"
                    : "border-line hover:border-line-strong"
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-mono text-xs">
                    {shortSha(deployment.commit_sha)}
                  </span>
                  <StatusBadge status={deployment.status} />
                </div>
                <div className="text-faint mt-1.5 flex items-center justify-between gap-2 text-xs">
                  <span>
                    {deployment.rolled_back_from
                      ? "Rolled back"
                      : timeAgo(deployment.created_at)}
                  </span>
                  <span className="tnum font-mono">
                    {formatDuration(seconds)}
                  </span>
                </div>
              </button>
            </li>
          );
        })}
      </ul>

      <div className="min-w-0 space-y-3">
        <Panel className="min-w-0 overflow-hidden">
          {active ? (
            <LogView key={active} appId={appId} deploymentId={active} />
          ) : (
            <p className="text-faint p-4 text-[13px]">
              Select a deployment to read its build log.
            </p>
          )}
        </Panel>

        {canRollback && activeDeployment && (
          <div className="flex flex-wrap items-center gap-3">
            <Button
              onClick={() => rollback.mutate(activeDeployment.id)}
              disabled={rollback.isPending}
            >
              {rollback.isPending
                ? "Rolling back…"
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
