"use client";

import { useQuery } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import {
  api,
  formatDuration,
  shortSha,
  successRate,
  timeAgo,
  type ProjectRollup,
  type RecentDeployment,
} from "@/lib/api";
import { BuildDurationChart, DeployActivityChart } from "@/components/charts";
import {
  Empty,
  ErrorText,
  Panel,
  PageHeader,
  Section,
  SegmentedControl,
  Skeleton,
  Stat,
  StatRow,
  StatusBadge,
} from "@/components/ui";

const WINDOWS = [
  { value: 7, label: "7 days" },
  { value: 30, label: "30 days" },
  { value: 90, label: "90 days" },
];

export default function AnalyticsPage() {
  const [days, setDays] = useState(30);

  // One request covers the whole page: the rollups are computed in Postgres,
  // so widening the window costs a wider aggregate rather than more rows on
  // the wire.
  const stats = useQuery({
    queryKey: ["stats", days],
    queryFn: () => api.stats(days),
    refetchInterval: 30_000,
    // Keeps the previous window on screen while the new one loads, so
    // switching the range does not blank the page.
    placeholderData: (previous) => previous,
  });

  const data = stats.data;
  const rate = data
    ? successRate(data.summary.succeeded, data.summary.failed)
    : null;
  const shipped = data ? data.summary.total > 0 : false;

  return (
    <main className="mx-auto max-w-6xl px-8 py-10">
      <PageHeader
        title="Analytics"
        description={`Deploy activity across every project, last ${days} days.`}
        actions={
          <SegmentedControl
            label="Time range"
            options={WINDOWS}
            value={days}
            onChange={setDays}
          />
        }
      />

      {stats.error && (
        <div className="mt-6">
          <ErrorText>Could not load analytics. Retrying shortly.</ErrorText>
        </div>
      )}

      {!data ? (
        <LoadingShell />
      ) : !shipped ? (
        <div className="mt-8">
          <Empty
            title={`Nothing deployed in the last ${days} days`}
            hint="Deploy a project and its build history will collect here."
          />
        </div>
      ) : (
        <>
          {/* The hero is the rhythm of shipping, not a headline figure. */}
          <Panel className="mt-8 overflow-hidden">
            <div className="border-line flex flex-wrap items-baseline justify-between gap-2 border-b px-5 py-3.5">
              <h2 className="text-[13px] font-semibold tracking-tight">
                Deploy activity
              </h2>
              <p className="text-faint text-xs">
                Bar height is deploys that day, split by outcome
              </p>
            </div>
            <div className="px-3 py-4">
              <DeployActivityChart data={data.daily} />
            </div>
          </Panel>

          <div className="mt-3">
            <StatRow>
              <Stat
                label="Deployments"
                value={data.summary.total}
                detail={
                  data.summary.in_flight > 0
                    ? `${data.summary.in_flight} running now`
                    : `${(data.summary.total / days).toFixed(1)} per day`
                }
              />
              <Stat
                label="Success rate"
                value={rate === null ? "--" : `${rate}%`}
                tone={
                  rate !== null && rate < 80 ? "text-danger" : "text-success"
                }
                detail={`${data.summary.failed} failed`}
              />
              <Stat
                label="Median build"
                value={formatDuration(data.summary.median_build_seconds)}
                detail={`95th percentile ${formatDuration(
                  data.summary.p95_build_seconds,
                )}`}
              />
              <Stat
                label="Projects"
                value={data.projects.length}
                detail={`${
                  data.projects.filter((p) => p.deploys > 0).length
                } deployed in window`}
              />
            </StatRow>
          </div>

          <Section
            title="Build time"
            description="Median across all projects, per day. A gap is a day nothing was built."
          >
            <BuildDurationChart data={data.daily} />
          </Section>

          <Section
            title="Per project"
            description="Every project you own, whether or not it shipped"
          >
            <ProjectTable projects={data.projects} />
          </Section>

          <Section
            title="Latest deployments"
            description="Most recent first, across all projects"
          >
            <ActivityFeed deployments={data.recent} />
          </Section>
        </>
      )}
    </main>
  );
}

function ProjectTable({ projects }: { projects: ProjectRollup[] }) {
  return (
    <Panel className="overflow-hidden">
      <div className="overflow-x-auto">
        <table className="w-full min-w-[42rem] text-[13px]">
          <thead>
            <tr className="border-line text-faint border-b text-xs">
              <th className="px-5 py-2.5 text-left font-medium">Project</th>
              <th className="px-5 py-2.5 text-right font-medium">Deploys</th>
              <th className="px-5 py-2.5 text-right font-medium">Succeeded</th>
              <th className="px-5 py-2.5 text-right font-medium">Failed</th>
              <th className="px-5 py-2.5 text-right font-medium">
                Median build
              </th>
              <th className="px-5 py-2.5 text-right font-medium">Last deploy</th>
            </tr>
          </thead>
          <tbody className="divide-line divide-y">
            {projects.map((project) => {
              const rate = successRate(project.succeeded, project.failed);
              return (
                <tr key={project.app_id} className="hover:bg-line/25">
                  <td className="px-5 py-3">
                    <Link
                      href={`/projects/${project.app_id}`}
                      className="hover:text-accent font-medium transition-colors"
                    >
                      {project.name}
                    </Link>
                  </td>
                  <td className="tnum px-5 py-3 text-right font-mono">
                    {project.deploys}
                  </td>
                  <td className="tnum px-5 py-3 text-right font-mono">
                    {project.succeeded > 0 ? (
                      <span className="text-success">{project.succeeded}</span>
                    ) : (
                      <span className="text-faint">0</span>
                    )}
                    {rate !== null && (
                      <span className="text-faint ml-2 text-xs">{rate}%</span>
                    )}
                  </td>
                  <td className="tnum px-5 py-3 text-right font-mono">
                    {project.failed > 0 ? (
                      <span className="text-danger">{project.failed}</span>
                    ) : (
                      <span className="text-faint">0</span>
                    )}
                  </td>
                  <td className="tnum text-muted px-5 py-3 text-right font-mono">
                    {formatDuration(project.median_build_seconds)}
                  </td>
                  <td className="text-muted px-5 py-3 text-right">
                    {project.last_deploy_at ? (
                      timeAgo(project.last_deploy_at)
                    ) : (
                      <span className="text-faint">never</span>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </Panel>
  );
}

function ActivityFeed({ deployments }: { deployments: RecentDeployment[] }) {
  if (deployments.length === 0) {
    return <Empty title="No deployments yet" />;
  }

  return (
    <Panel className="divide-line divide-y overflow-hidden">
      {deployments.map((deployment) => (
        <Link
          key={deployment.id}
          href={`/projects/${deployment.app_id}`}
          className="hover:bg-line/25 flex items-center gap-4 px-5 py-3 transition-colors"
        >
          <StatusBadge status={deployment.status} showLabel={false} />

          <span className="min-w-0 flex-1 truncate text-[13px] font-medium">
            {deployment.app_name}
          </span>

          <span className="text-muted hidden font-mono text-xs sm:inline">
            {shortSha(deployment.commit_sha)}
          </span>

          {deployment.rolled_back && (
            <span className="border-line text-muted rounded border px-1.5 py-0.5 text-xs">
              rollback
            </span>
          )}

          <span className="tnum text-faint w-16 text-right font-mono text-xs">
            {formatDuration(deployment.duration_seconds)}
          </span>

          <span className="text-faint w-16 text-right text-xs">
            {timeAgo(deployment.created_at)}
          </span>
        </Link>
      ))}
    </Panel>
  );
}

/** Holds the page's shape while the first request is in flight. */
function LoadingShell() {
  return (
    <div className="mt-8 space-y-3">
      <Panel className="p-5">
        <Skeleton className="h-4 w-32" />
        <Skeleton className="mt-5 h-56 w-full" />
      </Panel>
      <Panel className="grid gap-6 p-5 sm:grid-cols-4">
        {[0, 1, 2, 3].map((i) => (
          <div key={i}>
            <Skeleton className="h-3 w-20" />
            <Skeleton className="mt-3 h-6 w-14" />
          </div>
        ))}
      </Panel>
    </div>
  );
}
