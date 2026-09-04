"use client";

import { useQuery } from "@tanstack/react-query";
import { api } from "@/lib/api";
import { Card, Label } from "@/components/ui";

export default function SettingsPage() {
  const me = useQuery({ queryKey: ["me"], queryFn: () => api.me() });

  return (
    <main className="mx-auto max-w-3xl px-8 py-10">
      <h1 className="text-xl font-semibold tracking-tight">Settings</h1>

      <Card className="mt-6 space-y-4 p-5">
        <div className="space-y-1.5">
          <Label>Signed in as</Label>
          <p className="text-sm">{me.data?.email ?? "..."}</p>
        </div>
        <div className="space-y-1.5">
          <Label>Account created</Label>
          <p className="text-muted text-sm">
            {me.data ? new Date(me.data.created_at).toLocaleString() : "..."}
          </p>
        </div>
      </Card>
    </main>
  );
}
