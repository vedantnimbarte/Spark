"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { PlusIcon, TrashIcon } from "@/components/icons";
import { Button, Card, ErrorText, Input, Label } from "@/components/ui";

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
      <Card className="p-5">
        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            add.mutate();
          }}
        >
          <div className="space-y-1.5">
            <Label>Custom domain</Label>
            <Input
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="app.example.com"
            />
          </div>

          {error && <ErrorText>{error}</ErrorText>}

          <Button type="submit" variant="primary" disabled={add.isPending}>
            <PlusIcon className="size-4" />
            {add.isPending ? "Adding…" : "Add domain"}
          </Button>
        </form>
      </Card>

      <Card className="divide-border divide-y">
        <div className="flex items-center justify-between px-4 py-3">
          <div>
            <p className="text-sm">{appName}.localhost</p>
            <p className="text-faint mt-0.5 text-xs">
              Generated automatically
            </p>
          </div>
          <span className="text-faint text-xs">HTTP</span>
        </div>

        {domains.data?.map((d) => (
          <div
            key={d.id}
            className="flex items-center justify-between gap-4 px-4 py-3"
          >
            <div className="min-w-0">
              <p className="truncate text-sm">{d.domain_name}</p>
              <p className="text-faint mt-0.5 text-xs">
                Point a DNS record at this cluster to use it.
              </p>
            </div>
            <div className="flex shrink-0 items-center gap-3">
              {/* TLS is deferred in v1; ssl_status is always "none". */}
              <span className="text-faint text-xs">HTTP</span>
              <Button
                variant="danger"
                aria-label={`Remove ${d.domain_name}`}
                onClick={() => remove.mutate(d.id)}
                disabled={remove.isPending}
              >
                <TrashIcon className="size-3.5" />
              </Button>
            </div>
          </div>
        ))}
      </Card>
    </div>
  );
}
