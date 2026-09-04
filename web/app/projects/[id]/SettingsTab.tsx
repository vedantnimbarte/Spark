"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { api, ApiError, type Application } from "@/lib/api";
import { Button, Card, ErrorText, Input, Label } from "@/components/ui";

export function SettingsTab({ app }: { app: Application }) {
  const router = useRouter();
  const queryClient = useQueryClient();

  const [branch, setBranch] = useState(app.git_branch);
  const [dockerfile, setDockerfile] = useState(app.dockerfile_path);
  const [port, setPort] = useState(String(app.container_port));
  const [cpu, setCpu] = useState(app.cpu_limit);
  const [memory, setMemory] = useState(app.memory_limit);
  const [replicas, setReplicas] = useState(String(app.replicas));
  const [token, setToken] = useState("");
  const [confirm, setConfirm] = useState("");

  const webhook = useQuery({
    queryKey: ["webhook", app.id],
    queryFn: () => api.webhook(app.id),
  });

  const save = useMutation({
    mutationFn: () =>
      api.updateApp(app.id, {
        git_branch: branch,
        dockerfile_path: dockerfile,
        container_port: Number(port),
        cpu_limit: cpu,
        memory_limit: memory,
        replicas: Number(replicas),
      }),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["app", app.id] }),
  });

  const destroy = useMutation({
    mutationFn: () => api.deleteApp(app.id),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: ["apps"] });
      router.push("/projects");
    },
  });

  const saveToken = useMutation({
    mutationFn: () => api.setGitCredentials(app.id, token),
    onSuccess: async () => {
      setToken("");
      await queryClient.invalidateQueries({ queryKey: ["app", app.id] });
    },
  });

  const clearToken = useMutation({
    mutationFn: () => api.clearGitCredentials(app.id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["app", app.id] }),
  });

  const error = save.error instanceof ApiError ? save.error.message : null;
  const tokenError =
    saveToken.error instanceof ApiError ? saveToken.error.message : null;
  const origin = typeof window === "undefined" ? "" : window.location.origin;

  return (
    <div className="space-y-6">
      <Card className="p-5">
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            save.mutate();
          }}
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label>Branch</Label>
              <Input
                value={branch}
                onChange={(e) => setBranch(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>Dockerfile path</Label>
              <Input
                value={dockerfile}
                onChange={(e) => setDockerfile(e.target.value)}
                className="font-mono"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Container port</Label>
              <Input
                type="number"
                min={1}
                max={65535}
                value={port}
                onChange={(e) => setPort(e.target.value)}
              />
            </div>
            <div className="space-y-1.5">
              <Label>CPU limit</Label>
              <Input
                value={cpu}
                onChange={(e) => setCpu(e.target.value)}
                placeholder="500m"
                className="font-mono"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Memory limit</Label>
              <Input
                value={memory}
                onChange={(e) => setMemory(e.target.value)}
                placeholder="512Mi"
                className="font-mono"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Replicas</Label>
              <Input
                type="number"
                min={0}
                max={10}
                value={replicas}
                onChange={(e) => setReplicas(e.target.value)}
              />
              <p className="text-faint text-xs">
                0 stops the application without deleting it.
              </p>
            </div>
          </div>

          {error && <ErrorText>{error}</ErrorText>}

          <div className="flex items-center gap-3">
            <Button type="submit" variant="primary" disabled={save.isPending}>
              {save.isPending ? "Saving…" : "Save"}
            </Button>
            <p className="text-faint text-xs">
              Limits apply on the next deployment.
            </p>
          </div>
        </form>
      </Card>

      <Card className="space-y-3 p-5">
        <div>
          <h2 className="text-sm font-medium">Private repository access</h2>
          <p className="text-muted mt-1 text-xs">
            A personal access token with read access. Stored in a Kubernetes
            Secret separate from your environment variables, so it is never
            visible to the running application.
          </p>
        </div>

        {app.git_credentials_set ? (
          <div className="flex flex-wrap items-center gap-3">
            <span className="text-success text-sm">A token is configured</span>
            <Button
              variant="danger"
              onClick={() => clearToken.mutate()}
              disabled={clearToken.isPending}
            >
              {clearToken.isPending ? "Removing..." : "Remove token"}
            </Button>
          </div>
        ) : (
          <form
            className="space-y-3"
            onSubmit={(e) => {
              e.preventDefault();
              saveToken.mutate();
            }}
          >
            <Input
              type="password"
              value={token}
              onChange={(e) => setToken(e.target.value)}
              placeholder="ghp_..."
              className="font-mono"
            />
            {tokenError && <ErrorText>{tokenError}</ErrorText>}
            <Button
              type="submit"
              variant="primary"
              disabled={!token || saveToken.isPending}
            >
              {saveToken.isPending ? "Saving..." : "Save token"}
            </Button>
          </form>
        )}
      </Card>

      <Card className="space-y-3 p-5">
        <div>
          <h2 className="text-sm font-medium">Push webhook</h2>
          <p className="text-muted mt-1 text-xs">
            Add this to your repository so a push deploys automatically. The
            secret signs the payload.
          </p>
        </div>

        <div className="space-y-1.5">
          <Label>GitHub payload URL</Label>
          <Input
            readOnly
            value={webhook.data ? origin + webhook.data.github_url : "…"}
            className="font-mono"
            onFocus={(e) => e.currentTarget.select()}
          />
        </div>

        <div className="space-y-1.5">
          <Label>Secret</Label>
          <Input
            readOnly
            value={webhook.data?.secret ?? "…"}
            className="font-mono"
            onFocus={(e) => e.currentTarget.select()}
          />
        </div>
      </Card>

      <Card className="border-danger/30 space-y-3 p-5">
        <div>
          <h2 className="text-danger text-sm font-medium">Delete project</h2>
          <p className="text-muted mt-1 text-xs">
            Removes the application and its entire Kubernetes namespace,
            including environment variables. This cannot be undone.
          </p>
        </div>

        <div className="flex flex-wrap items-center gap-2">
          <Input
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            placeholder={`Type ${app.name} to confirm`}
            className="max-w-xs"
          />
          <Button
            variant="danger"
            disabled={confirm !== app.name || destroy.isPending}
            onClick={() => destroy.mutate()}
          >
            {destroy.isPending ? "Deleting…" : "Delete permanently"}
          </Button>
        </div>
      </Card>
    </div>
  );
}
