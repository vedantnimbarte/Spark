"use client";

import { useMutation } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { api, ApiError } from "@/lib/api";
import { SparkIcon } from "@/components/icons";
import { Button, ErrorText, Input, Label } from "@/components/ui";

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
        ? "Something went wrong"
        : null;

  return (
    <main className="flex min-h-screen items-center justify-center px-4">
      <div className="w-full max-w-sm">
        <div className="mb-8 flex items-center gap-2">
          <SparkIcon className="text-accent size-5" />
          <h1 className="text-lg font-semibold tracking-tight">Spark</h1>
        </div>

        <form
          className="space-y-4"
          onSubmit={(e) => {
            e.preventDefault();
            submit.mutate();
          }}
        >
          <div className="space-y-1.5">
            <Label>Email</Label>
            <Input
              type="email"
              autoComplete="email"
              required
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="you@example.com"
            />
          </div>

          <div className="space-y-1.5">
            <Label>Password</Label>
            <Input
              type="password"
              autoComplete={
                mode === "login" ? "current-password" : "new-password"
              }
              required
              minLength={mode === "signup" ? 12 : undefined}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={
                mode === "signup" ? "at least 12 characters" : "••••••••"
              }
            />
          </div>

          {error && <ErrorText>{error}</ErrorText>}

          <Button
            type="submit"
            variant="primary"
            className="w-full"
            disabled={submit.isPending}
          >
            {submit.isPending
              ? "…"
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
