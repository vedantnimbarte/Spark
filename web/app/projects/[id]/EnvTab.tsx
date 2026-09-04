"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { Button, Card, Empty, ErrorText, Input, Label } from "@/components/ui";

export function EnvTab({ appId }: { appId: string }) {
  const queryClient = useQueryClient();
  const [key, setKey] = useState("");
  const [value, setValue] = useState("");
  // Values are never listed, only revealed one at a time on request.
  const [revealed, setRevealed] = useState<Record<string, string>>({});

  const env = useQuery({
    queryKey: ["env", appId],
    queryFn: () => api.listEnv(appId),
  });

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["env", appId] });

  const save = useMutation({
    mutationFn: () => api.setEnv(appId, { [key]: value }),
    onSuccess: async () => {
      setKey("");
      setValue("");
      await invalidate();
    },
  });

  const remove = useMutation({
    mutationFn: (k: string) => api.deleteEnv(appId, k),
    onSuccess: invalidate,
  });

  const reveal = useMutation({
    mutationFn: (k: string) => api.revealEnv(appId, k),
    onSuccess: (data) =>
      setRevealed((current) => ({ ...current, [data.key]: data.value })),
  });

  const error = save.error instanceof ApiError ? save.error.message : null;

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
              <Label>Key</Label>
              <Input
                required
                value={key}
                onChange={(e) => setKey(e.target.value)}
                placeholder="DATABASE_URL"
                className="font-mono"
              />
            </div>
            <div className="space-y-1.5">
              <Label>Value</Label>
              <Input
                required
                value={value}
                onChange={(e) => setValue(e.target.value)}
                placeholder="postgres://…"
                className="font-mono"
              />
            </div>
          </div>

          {error && <ErrorText>{error}</ErrorText>}

          <div className="flex items-center gap-3">
            <Button type="submit" variant="primary" disabled={save.isPending}>
              <PlusIcon className="size-4" />
              {save.isPending ? "Saving…" : "Add variable"}
            </Button>
            <p className="text-faint text-xs">
              Saving restarts the application so the value takes effect.
            </p>
          </div>
        </form>
      </Card>

      {env.data?.keys.length === 0 && (
        <Empty
          title="No environment variables"
          hint="Values are stored only as a Kubernetes Secret, never in the database."
        />
      )}

      {env.data && env.data.keys.length > 0 && (
        <Card className="divide-border divide-y">
          {env.data.keys.map((k) => (
            <div
              key={k}
              className="flex items-center justify-between gap-4 px-4 py-3"
            >
              <div className="min-w-0">
                <p className="truncate font-mono text-sm">{k}</p>
                <p className="text-faint mt-0.5 truncate font-mono text-xs">
                  {revealed[k] ?? "••••••••"}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {revealed[k] === undefined && (
                  <Button
                    onClick={() => reveal.mutate(k)}
                    disabled={reveal.isPending}
                  >
                    Reveal
                  </Button>
                )}
                <Button
                  variant="danger"
                  aria-label={`Delete ${k}`}
                  onClick={() => remove.mutate(k)}
                  disabled={remove.isPending}
                >
                  <TrashIcon className="size-3.5" />
                </Button>
              </div>
            </div>
          ))}
        </Card>
      )}
    </div>
  );
}
