"use client";

import { useMutation } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { SparkIcon } from "@/components/icons";
import { Button, ErrorText, Field, Input } from "@/components/ui";

export default function LoginPage() {
  const router = useRouter();
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  // The API opens signup only while the instance has no users, so the form
  // offers both and lets the server decide.
  const [mode, setMode] = useState<"login" | "signup">("login");

  const submit = useMutation({
    mutationFn: () =>
      mode === "login"
        ? api.login(email, password)
        : api.signup(email, password),
    onSuccess: () => router.push("/projects"),
  });

  const error =
    submit.error instanceof ApiError
      ? submit.error.message
      : submit.error
        ? "Could not reach the control plane"
        : null;

  return (
    <main className="flex min-h-screen items-center justify-center px-4">
      {/* The rule under the mark is the only ornament on this page: it gives
          the form a top edge without boxing it. */}
      <div className="w-full max-w-sm">
        <div className="border-line flex items-center gap-2.5 border-b pb-5">
          <SparkIcon className="text-accent size-5" />
          <h1 className="text-[15px] font-semibold tracking-tight">Spark</h1>
          <span className="text-faint ml-auto text-xs">Control plane</span>
        </div>

        <h2 className="mt-8 text-[22px] leading-tight font-semibold tracking-tight">
          {mode === "login" ? "Sign in" : "Create the first account"}
        </h2>
        <p className="text-muted mt-1.5 text-[13px]">
          {mode === "login"
            ? "Deploy and watch the applications on your cluster."
            : "This account owns the instance. Signup closes once it exists."}
        </p>

        <form
          className="mt-7 space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            submit.mutate();
          }}
        >
          <Field label="Email" htmlFor="email">
            <Input
              id="email"
              type="email"
              autoComplete="email"
              required
              autoFocus
              value={email}
              onChange={(event) => setEmail(event.target.value)}
              placeholder="you@example.com"
            />
          </Field>

          <Field
            label="Password"
            htmlFor="password"
            hint={mode === "signup" ? "At least 12 characters" : undefined}
          >
            <Input
              id="password"
              type="password"
              autoComplete={
                mode === "login" ? "current-password" : "new-password"
              }
              required
              minLength={mode === "signup" ? 12 : undefined}
              value={password}
              onChange={(event) => setPassword(event.target.value)}
              placeholder="••••••••••••"
            />
          </Field>

          {error && <ErrorText>{error}</ErrorText>}

          <Button
            type="submit"
            variant="primary"
            className="w-full"
            disabled={submit.isPending}
          >
            {submit.isPending
              ? mode === "login"
                ? "Signing in…"
                : "Creating…"
              : mode === "login"
                ? "Sign in"
                : "Create account"}
          </Button>
        </form>

        <button
          onClick={() => setMode(mode === "login" ? "signup" : "login")}
          className="text-faint hover:text-muted mt-6 text-xs transition-colors"
        >
          {mode === "login"
            ? "First time here? Create the first account"
            : "Already have an account? Sign in"}
        </button>
      </div>
    </main>
  );
}
