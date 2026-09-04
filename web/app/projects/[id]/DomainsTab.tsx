"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { Button, ErrorText, Field, Input, Panel } from "@/components/ui";

export function DomainsTab({
  appId,
  appName,
}: {
  appId: string;
  appName: string;
}) {
  const queryClient = useQueryClient();
  const [name, setName] = useState("");

  const domains = useQuery({
    queryKey: ["domains", appId],
    queryFn: () => api.listDomains(appId),
  });

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: ["domains", appId] });

  const add = useMutation({
    mutationFn: () => api.addDomain(appId, name),
    onSuccess: async () => {
      setName("");
      await invalidate();
    },
  });

  const remove = useMutation({
    mutationFn: (id: string) => api.deleteDomain(appId, id),
    onSuccess: invalidate,
  });

  const error = add.error instanceof ApiError ? add.error.message : null;

  return (
    <div className="space-y-6">
      <Panel className="p-5">
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            add.mutate();
          }}
        >
          <Field
            label="Custom domain"
            htmlFor="domain-name"
            hint="Point a CNAME or A record at this cluster's ingress first, or the domain will resolve nowhere."
          >
            <Input
              id="domain-name"
              required
              value={name}
              onChange={(event) => setName(event.target.value)}
              placeholder="app.example.com"
              className="font-mono"
            />
          </Field>

          {error && <ErrorText>{error}</ErrorText>}

          <Button type="submit" variant="primary" disabled={add.isPending}>
            <PlusIcon className="size-4" />
            {add.isPending ? "Adding…" : "Add domain"}
          </Button>
        </form>
      </Panel>

      <Panel className="divide-line divide-y overflow-hidden">
        <div className="flex items-center justify-between gap-4 px-5 py-3.5">
          <div className="min-w-0">
            <p className="truncate font-mono text-[13px]">
              {appName}.localhost
            </p>
            <p className="text-faint mt-0.5 text-xs">
              Created with the project. It cannot be removed.
            </p>
          </div>
          <span className="border-line text-faint shrink-0 rounded border px-1.5 py-0.5 text-xs">
            HTTP
          </span>
        </div>

        {domains.data?.map((domain) => (
          <div
            key={domain.id}
            className="flex items-center justify-between gap-4 px-5 py-3.5"
          >
            <div className="min-w-0">
              <p className="truncate font-mono text-[13px]">
                {domain.domain_name}
              </p>
              <p className="text-faint mt-0.5 text-xs">
                Added {new Date(domain.created_at).toLocaleDateString()}
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-3">
              {/* TLS is deferred in v1; ssl_status is always "none". */}
              <span className="border-line text-faint rounded border px-1.5 py-0.5 text-xs">
                HTTP
              </span>
              <Button
                variant="danger"
                aria-label={`Remove ${domain.domain_name}`}
                onClick={() => remove.mutate(domain.id)}
                disabled={remove.isPending}
              >
                <TrashIcon className="size-3.5" />
              </Button>
            </div>
          </div>
        ))}
      </Panel>

      {remove.error instanceof ApiError && (
        <ErrorText>{remove.error.message}</ErrorText>
      )}
    </div>
  );
}
