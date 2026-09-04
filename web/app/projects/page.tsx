"use client";

import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import Link from "next/link";
import { useState } from "react";
import {
  api,
  ApiError,
  timeAgo,
  shortSha,
  type Application,
  type Deployment,
} from "@/lib/api";
import { BranchIcon, PlusIcon } from "@/components/icons";
import {
  Button,
  Card,
  Empty,
  ErrorText,
  Input,
  Label,
  StatusBadge,
} from "@/components/ui";

export default function ProjectsPage() {
  const [creating, setCreating] = useState(false);

  const apps = useQuery({ queryKey: ["apps"], queryFn: () => api.listApps() });

  // One query per project for its latest deployment. Kept separate from the
  // list so a slow cluster read never delays the grid itself.
  const latest = useQueries({
    queries: (apps.data ?? []).map((app) => ({
      queryKey: ["deployments", app.id],
      queryFn: () => api.listDeployments(app.id),
      refetchInterval: 5_000,
    })),
  });

  return (
    <main className="mx-auto max-w-6xl px-8 py-10">
      <header className="mb-8 flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold tracking-tight">Projects</h1>
          <p className="text-muted mt-1 text-sm">
            {apps.data?.length ?? 0} application
            {apps.data?.length === 1 ? "" : "s"}
          </p>
        </div>
        <Button variant="primary" onClick={() => setCreating(true)}>
          <PlusIcon className="size-4" />
          New project
        </Button>
      </header>

      {creating && <CreateForm onDone={() => setCreating(false)} />}

      {apps.isPending && <p className="text-faint text-sm">Loading…</p>}

      {apps.data?.length === 0 && !creating && (
        <Empty
          title="No projects yet"
          hint="Create one from a Git repository containing a Dockerfile."
        />
      )}

      <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
        {apps.data?.map((app, index) => (
          <ProjectCard
            key={app.id}
            app={app}
            deployments={latest[index]?.data}
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

  return (
    <Link href={`/projects/${app.id}`} className="group block">
      <Card className="hover:border-border-strong h-full p-4 transition-colors">
        <div className="flex items-start justify-between gap-2">
          <span className="truncate text-sm font-medium">{app.name}</span>
          {last ? (
            <StatusBadge status={last.status} showLabel={false} />
          ) : (
            <span className="text-faint text-xs">never deployed</span>
          )}
        </div>

        <p className="text-faint mt-3 flex items-center gap-1.5 text-xs">
          <BranchIcon className="size-3.5 shrink-0" />
          <span className="truncate">{app.git_branch}</span>
        </p>

        <div className="text-faint mt-4 flex items-center justify-between text-xs">
          <span className="font-mono">
            {last ? shortSha(last.commit_sha) : "—"}
          </span>
          <span>{last ? timeAgo(last.created_at) : ""}</span>
        </div>
      </Card>
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

  const error =
    create.error instanceof ApiError ? create.error.message : null;

  return (
    <Card className="mb-6 p-5">
      <form
        className="space-y-4"
        onSubmit={(e) => {
          e.preventDefault();
          create.mutate();
        }}
      >
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-1.5">
            <Label>Name</Label>
            <Input
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="my-app"
            />
            <p className="text-faint text-xs">
              Published at {name || "my-app"}.localhost
            </p>
          </div>

          <div className="space-y-1.5">
            <Label>Git repository</Label>
            <Input
              required
              value={gitRepo}
              onChange={(e) => setGitRepo(e.target.value)}
              placeholder="https://github.com/you/repo.git"
            />
          </div>

          <div className="space-y-1.5">
            <Label>Branch</Label>
            <Input
              required
              value={branch}
              onChange={(e) => setBranch(e.target.value)}
            />
          </div>

          <div className="space-y-1.5">
            <Label>Container port</Label>
            <Input
              required
              type="number"
              min={1}
              max={65535}
              value={port}
              onChange={(e) => setPort(e.target.value)}
            />
          </div>
        </div>

        {error && <ErrorText>{error}</ErrorText>}

        <div className="flex gap-2">
          <Button type="submit" variant="primary" disabled={create.isPending}>
            {create.isPending ? "Creating…" : "Create"}
          </Button>
          <Button type="button" onClick={onDone}>
            Cancel
          </Button>
        </div>
      </form>
    </Card>
  );
}
