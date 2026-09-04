"use client";

import {
  useMutation,
  useQueries,
  useQuery,
  useQueryClient,
} from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import {
  api,
  ApiError,
  shortSha,
  successRate,
  timeAgo,
  type Application,
  type Deployment,
} from "@/lib/api";
import { BranchIcon, PlusIcon } from "@/components/icons";

import {
  ActivityLine,
  Button,
  Empty,
  ErrorText,
  Field,
  Input,
  isInFlight,
  OutcomeSparkline,
  Panel,
  PageHeader,
  Skeleton,
  StatusBadge,
} from "@/components/ui";

export default function ProjectsPage() {
  const [creating, setCreating] = useState(false);

  const apps = useQuery({ queryKey: ["apps"], queryFn: () => api.listApps() });

  // One query per project for its deployment history. Kept separate from the
  // list so a slow read never delays the grid itself.
  const histories = useQueries({
    queries: (apps.data ?? []).map((app) => ({
      queryKey: ["deployments", app.id],
      queryFn: () => api.listDeployments(app.id),
      refetchInterval: 5_000,
    })),
  });

  const count = apps.data?.length ?? 0;
  const building = histories.filter((query) =>
    query.data?.[0] ? isInFlight(query.data[0].status) : false,
  ).length;

  return (
    <main className="mx-auto max-w-6xl px-8 py-10">
      <PageHeader
        title="Projects"
        description={
          apps.data
            ? building > 0
              ? `${count} application${count === 1 ? "" : "s"} · ${building} building now`
              : `${count} application${count === 1 ? "" : "s"}`
            : undefined
        }
        actions={
          <Button variant="primary" onClick={() => setCreating(true)}>
            <PlusIcon className="size-4" />
            New project
          </Button>
        }
      />

      {creating && (
        <div className="mt-6">
          <CreateForm onDone={() => setCreating(false)} />
        </div>
      )}

      {apps.isPending && (
        <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
          {[0, 1, 2].map((i) => (
            <Panel key={i} className="space-y-4 p-4">
              <Skeleton className="h-4 w-28" />
              <Skeleton className="h-3 w-20" />
              <Skeleton className="h-5 w-full" />
            </Panel>
          ))}
        </div>
      )}

      {apps.data?.length === 0 && !creating && (
        <div className="mt-8">
          <Empty
            title="No projects yet"
            hint="Point Spark at a Git repository containing a Dockerfile and it will build and run it on your cluster."
            action={
              <Button variant="primary" onClick={() => setCreating(true)}>
                <PlusIcon className="size-4" />
                New project
              </Button>
            }
          />
        </div>
      )}

      <div className="mt-8 grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {apps.data?.map((app, index) => (
          <ProjectCard
            key={app.id}
            app={app}
            deployments={histories[index]?.data}
          />
        ))}
      </div>
    </main>
  );
}

function ProjectCard({
  app,
  deployments,
}: {
  app: Application;
  deployments: Deployment[] | undefined;
}) {
  const last = deployments?.[0];
  const rate = deployments
    ? successRate(
        deployments.filter((d) => d.status === "deployed").length,
        deployments.filter((d) => d.status === "failed").length,
      )
    : null;

  return (
    <Link
      href={`/projects/${app.id}`}
      className="group focus-visible:outline-accent block rounded-lg"
    >
      <Panel className="hover:border-line-strong h-full overflow-hidden transition-colors">
        {/* Only a project with a build actually running shows motion. */}
        {last && isInFlight(last.status) ? (
          <ActivityLine />
        ) : (
          <div className="h-px" aria-hidden />
        )}

        <div className="p-4">
          <div className="flex items-start justify-between gap-2">
            <span className="group-hover:text-accent truncate text-[13px] font-semibold transition-colors">
              {app.name}
            </span>
            {last ? (
              <StatusBadge status={last.status} />
            ) : (
              <span className="text-faint text-xs">Never deployed</span>
            )}
          </div>

          <p className="text-faint mt-2.5 flex items-center gap-1.5 text-xs">
            <BranchIcon className="size-3.5 shrink-0" />
            <span className="truncate">{app.git_branch}</span>
            {last && (
              <>
                <span className="bg-line h-2.5 w-px" aria-hidden />
                <span className="truncate font-mono">
                  {shortSha(last.commit_sha)}
                </span>
              </>
            )}
          </p>

          <div className="mt-4">
            {deployments && deployments.length > 0 ? (
              <OutcomeSparkline deployments={deployments} />
            ) : (
              <div className="h-5" aria-hidden />
            )}
          </div>

          <div className="border-line text-faint mt-3 flex items-center justify-between border-t pt-3 text-xs">
            <span className="tnum">
              {rate === null ? "No finished builds" : `${rate}% succeeded`}
            </span>
            <span>{last ? timeAgo(last.created_at) : ""}</span>
          </div>
        </div>
      </Panel>
    </Link>
  );
}

function CreateForm({ onDone }: { onDone: () => void }) {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");
  const [gitRepo, setGitRepo] = useState("");
  const [branch, setBranch] = useState("main");
  const [port, setPort] = useState("8080");

  const create = useMutation({
    mutationFn: () =>
      api.createApp({
        name,
        git_repo: gitRepo,
        git_branch: branch,
        container_port: Number(port),
      }),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["apps"] });
      onDone();
    },
  });

  const error = create.error instanceof ApiError ? create.error.message : null;

  return (
    <Panel className="p-5">
      <h2 className="text-[13px] font-semibold tracking-tight">New project</h2>
      <p className="text-faint mt-0.5 text-xs">
        Spark clones the repository, builds the Dockerfile, and runs the image
        on your cluster.
      </p>

      <form
        className="mt-5 space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          create.mutate();
        }}
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <Field
            label="Name"
            htmlFor="project-name"
            hint={`Published at ${name || "my-app"}.localhost`}
          >
            <Input
              id="project-name"
              required
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="my-app"
            />
          </Field>

          <Field
            label="Git repository"
            htmlFor="project-repo"
            hint="Add a deploy token in Settings if it is private"
          >
            <Input
              id="project-repo"
              required
              value={gitRepo}
              onChange={(event) => setGitRepo(event.target.value)}
              placeholder="https://github.com/you/repo.git"
            />
          </Field>

          <Field label="Branch" htmlFor="project-branch">
            <Input
              id="project-branch"
              required
              value={branch}
              onChange={(event) => setBranch(event.target.value)}
            />
          </Field>

          <Field
            label="Container port"
            htmlFor="project-port"
            hint="The port your app listens on inside the container"
          >
            <Input
              id="project-port"
              required
              type="number"
              min={1}
              max={65535}
              value={port}
              onChange={(event) => setPort(event.target.value)}
            />
          </Field>
        </div>

        {error && <ErrorText>{error}</ErrorText>}

        <div className="flex gap-2">
          <Button type="submit" variant="primary" disabled={create.isPending}>
            {create.isPending ? "Creating…" : "Create project"}
          </Button>
          <Button type="button" onClick={onDone}>
            Cancel
          </Button>
        </div>
      </form>
    </Panel>
  );
}
