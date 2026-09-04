"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { PlusIcon, TrashIcon } from "@/components/icons";
import {
  Button,
  Empty,
  ErrorText,
  Field,
  Input,
  Panel,
  Skeleton,
} from "@/components/ui";

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
    onSuccess: async (_result, k) => {
      // A deleted key must not leave its value behind on screen.
      setRevealed((current) => {
        const next = { ...current };
        delete next[k];
        return next;
      });
      await invalidate();
    },
  });

  const reveal = useMutation({
    mutationFn: (k: string) => api.revealEnv(appId, k),
    onSuccess: (data) =>
      setRevealed((current) => ({ ...current, [data.key]: data.value })),
  });

  const hide = (k: string) =>
    setRevealed((current) => {
      const next = { ...current };
      delete next[k];
      return next;
    });

  const error = save.error instanceof ApiError ? save.error.message : null;

  return (
    <div className="space-y-6">
      <Panel className="p-5">
        <h2 className="text-[13px] font-semibold tracking-tight">
          Add a variable
        </h2>
        <p className="text-faint mt-0.5 text-xs">
          Values are written to this project&rsquo;s Kubernetes Secret. The
          control plane database stores only the key names.
        </p>

        <form
          className="mt-5 space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            save.mutate();
          }}
        >
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Key" htmlFor="env-key">
              <Input
                id="env-key"
                required
                value={key}
                onChange={(event) => setKey(event.target.value)}
                placeholder="DATABASE_URL"
                className="font-mono"
              />
            </Field>
            <Field label="Value" htmlFor="env-value">
              <Input
                id="env-value"
                required
                value={value}
                onChange={(event) => setValue(event.target.value)}
                placeholder="postgres://…"
                className="font-mono"
              />
            </Field>
          </div>

          {error && <ErrorText>{error}</ErrorText>}

          <div className="flex flex-wrap items-center gap-3">
            <Button type="submit" variant="primary" disabled={save.isPending}>
              <PlusIcon className="size-4" />
              {save.isPending ? "Saving…" : "Add variable"}
            </Button>
            <p className="text-faint text-xs">
              Saving restarts the application so the value takes effect.
            </p>
          </div>
        </form>
      </Panel>

      {env.isPending && (
        <Panel className="space-y-3 p-5">
          <Skeleton className="h-4 w-44" />
          <Skeleton className="h-4 w-32" />
        </Panel>
      )}

      {env.data?.keys.length === 0 && (
        <Empty
          title="No environment variables"
          hint="Anything you add here is injected into the container at startup."
        />
      )}

      {env.data && env.data.keys.length > 0 && (
        <Panel className="divide-line divide-y overflow-hidden">
          {env.data.keys.map((k) => (
            <div
              key={k}
              className="flex items-center justify-between gap-4 px-5 py-3"
            >
              <div className="min-w-0">
                <p className="truncate font-mono text-[13px]">{k}</p>
                <p
                  className={`mt-0.5 truncate font-mono text-xs ${
                    revealed[k] === undefined ? "text-faint" : "text-muted"
                  }`}
                >
                  {revealed[k] ?? "••••••••••••"}
                </p>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {revealed[k] === undefined ? (
                  <Button
                    onClick={() => reveal.mutate(k)}
                    disabled={reveal.isPending}
                  >
                    Reveal
                  </Button>
                ) : (
                  <Button onClick={() => hide(k)}>Hide</Button>
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
        </Panel>
      )}

      {remove.error instanceof ApiError && (
        <ErrorText>{remove.error.message}</ErrorText>
      )}
    </div>
  );
}
