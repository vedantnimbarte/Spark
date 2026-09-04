"use client";

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { api, ApiError, timeAgo, type SessionInfo } from "@/lib/api";
import {
  Button,
  Detail,
  Empty,
  ErrorText,
  Field,
  Input,
  Panel,
  PageHeader,
  Section,
  Skeleton,
  SuccessText,
} from "@/components/ui";

export default function SettingsPage() {
  const me = useQuery({ queryKey: ["me"], queryFn: () => api.me() });

  return (
    <main className="mx-auto max-w-3xl px-8 py-10">
      <PageHeader
        title="Settings"
        description="Your account and the sessions signed in to it."
      />

      <Section title="Account">
        <Panel className="grid gap-5 p-5 sm:grid-cols-2">
          <Detail label="Email">
            {me.data ? (
              <span className="truncate">{me.data.email}</span>
            ) : (
              <Skeleton className="h-4 w-40" />
            )}
          </Detail>
          <Detail label="Member since">
            {me.data ? (
              <span className="text-muted">
                {new Date(me.data.created_at).toLocaleDateString(undefined, {
                  day: "numeric",
                  month: "long",
                  year: "numeric",
                })}
              </span>
            ) : (
              <Skeleton className="h-4 w-32" />
            )}
          </Detail>
        </Panel>
      </Section>

      <Section
        title="Password"
        description="Changing it signs out every other session"
      >
        <PasswordForm />
      </Section>

      <Section
        title="Active sessions"
        description="Sign out a browser you no longer have in front of you"
      >
        <SessionList />
      </Section>
    </main>
  );
}

function PasswordForm() {
  const queryClient = useQueryClient();
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const [mismatch, setMismatch] = useState(false);

  const change = useMutation({
    mutationFn: () => api.changePassword(current, next),
    onSuccess: async () => {
      setCurrent("");
      setNext("");
      setConfirm("");
      // Other sessions were just dropped, so the list on this page is stale.
      await queryClient.invalidateQueries({ queryKey: ["sessions"] });
    },
  });

  const serverError =
    change.error instanceof ApiError
      ? change.error.message
      : change.error
        ? "Could not change the password"
        : null;

  return (
    <Panel className="p-5">
      <form
        className="space-y-4"
        onSubmit={(event) => {
          event.preventDefault();
          // Caught here rather than at the API, so the person is told which
          // field is wrong instead of being told the request failed.
          if (next !== confirm) {
            setMismatch(true);
            return;
          }
          setMismatch(false);
          change.mutate();
        }}
      >
        <Field label="Current password" htmlFor="current-password">
          <Input
            id="current-password"
            type="password"
            autoComplete="current-password"
            required
            value={current}
            onChange={(event) => setCurrent(event.target.value)}
          />
        </Field>

        <div className="grid gap-4 sm:grid-cols-2">
          <Field
            label="New password"
            htmlFor="new-password"
            hint="At least 12 characters"
          >
            <Input
              id="new-password"
              type="password"
              autoComplete="new-password"
              required
              minLength={12}
              value={next}
              onChange={(event) => setNext(event.target.value)}
            />
          </Field>

          <Field label="Confirm new password" htmlFor="confirm-password">
            <Input
              id="confirm-password"
              type="password"
              autoComplete="new-password"
              required
              value={confirm}
              onChange={(event) => setConfirm(event.target.value)}
            />
          </Field>
        </div>

        {mismatch && <ErrorText>The two new passwords do not match.</ErrorText>}
        {serverError && <ErrorText>{serverError}</ErrorText>}
        {change.isSuccess && (
          <SuccessText>
            Password changed.{" "}
            {change.data.sessions_revoked > 0
              ? `${change.data.sessions_revoked} other session${
                  change.data.sessions_revoked === 1 ? "" : "s"
                } signed out.`
              : "No other sessions were signed in."}
          </SuccessText>
        )}

        <Button type="submit" variant="primary" disabled={change.isPending}>
          {change.isPending ? "Changing…" : "Change password"}
        </Button>
      </form>
    </Panel>
  );
}

function SessionList() {
  const queryClient = useQueryClient();
  const sessions = useQuery({
    queryKey: ["sessions"],
    queryFn: () => api.listSessions(),
  });

  const revoke = useMutation({
    mutationFn: (id: string) => api.revokeSession(id),
    onSuccess: () =>
      queryClient.invalidateQueries({ queryKey: ["sessions"] }),
  });

  if (sessions.isPending) {
    return (
      <Panel className="space-y-3 p-5">
        <Skeleton className="h-4 w-48" />
        <Skeleton className="h-4 w-40" />
      </Panel>
    );
  }

  if (!sessions.data || sessions.data.length === 0) {
    return <Empty title="No active sessions" />;
  }

  return (
    <>
      <Panel className="divide-line divide-y overflow-hidden">
        {sessions.data.map((session) => (
          <SessionRow
            key={session.id}
            session={session}
            onRevoke={() => revoke.mutate(session.id)}
            revoking={revoke.isPending && revoke.variables === session.id}
          />
        ))}
      </Panel>
      {revoke.error instanceof ApiError && (
        <div className="mt-3">
          <ErrorText>{revoke.error.message}</ErrorText>
        </div>
      )}
    </>
  );
}

function SessionRow({
  session,
  onRevoke,
  revoking,
}: {
  session: SessionInfo;
  onRevoke: () => void;
  revoking: boolean;
}) {
  return (
    <div className="flex flex-wrap items-center gap-3 px-5 py-3.5">
      <div className="min-w-0 flex-1">
        <p className="text-[13px] font-medium">
          {session.current ? "This browser" : "Signed-in browser"}
        </p>
        <p className="text-faint mt-0.5 text-xs">
          Started {timeAgo(session.created_at)}, expires{" "}
          {new Date(session.expires_at).toLocaleDateString()}
        </p>
      </div>

      {session.current ? (
        // Revoking the current session would sign the person out from the
        // settings page, which reads as a bug rather than an action.
        <span className="border-accent/25 bg-accent/10 text-accent rounded-full border px-2.5 py-1 text-xs font-medium">
          Current
        </span>
      ) : (
        <Button variant="danger" onClick={onRevoke} disabled={revoking}>
          {revoking ? "Signing out…" : "Sign out"}
        </Button>
      )}
    </div>
  );
}
