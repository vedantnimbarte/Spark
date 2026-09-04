"use client";

import Link from "next/link";
import type { DeploymentStatus } from "@/lib/api";

/** Flat surface with a 1px border — no shadows, per the guidelines. */
export function Card({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`border border-border bg-surface rounded-md ${className}`}
    >
      {children}
    </div>
  );
}

type ButtonVariant = "primary" | "secondary" | "danger";

const BUTTON_STYLES: Record<ButtonVariant, string> = {
  primary: "bg-fg text-base hover:bg-fg/85 border-transparent",
  secondary: "bg-transparent text-fg border-border hover:border-border-strong",
  danger: "bg-transparent text-danger border-danger/40 hover:border-danger",
};

export function Button({
  variant = "secondary",
  className = "",
  ...props
}: React.ButtonHTMLAttributes<HTMLButtonElement> & {
  variant?: ButtonVariant;
}) {
  return (
    <button
      {...props}
      className={`inline-flex items-center justify-center gap-2 rounded-md border px-3 py-1.5 text-sm font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${BUTTON_STYLES[variant]} ${className}`}
    />
  );
}

export function Input({
  className = "",
  ...props
}: React.InputHTMLAttributes<HTMLInputElement>) {
  return (
    <input
      {...props}
      className={`w-full rounded-md border border-border bg-base px-3 py-1.5 text-sm text-fg transition-colors placeholder:text-faint focus:border-border-strong focus:outline-none ${className}`}
    />
  );
}

export function Label({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-muted block text-xs font-medium tracking-wide uppercase">
      {children}
    </span>
  );
}

const STATUS_COLOR: Record<DeploymentStatus, string> = {
  pending: "bg-muted",
  building: "bg-pending",
  deploying: "bg-pending",
  deployed: "bg-success",
  failed: "bg-danger",
};

const IN_FLIGHT: DeploymentStatus[] = ["pending", "building", "deploying"];

/** Colour carries the state; the label carries the meaning. */
export function StatusBadge({
  status,
  showLabel = true,
}: {
  status: DeploymentStatus;
  showLabel?: boolean;
}) {
  const active = IN_FLIGHT.includes(status);
  return (
    <span className="inline-flex items-center gap-2 text-xs">
      <span
        className={`size-1.5 rounded-full ${STATUS_COLOR[status]} ${active ? "animate-pulse-dot" : ""}`}
        aria-hidden
      />
      {showLabel && <span className="text-muted">{status}</span>}
      <span className="sr-only">{status}</span>
    </span>
  );
}

export function Empty({
  title,
  hint,
}: {
  title: string;
  hint?: React.ReactNode;
}) {
  return (
    <div className="border-border rounded-md border border-dashed px-6 py-12 text-center">
      <p className="text-sm text-muted">{title}</p>
      {hint && <div className="mt-2 text-xs text-faint">{hint}</div>}
    </div>
  );
}

export function ErrorText({ children }: { children: React.ReactNode }) {
  return <p className="text-danger text-sm">{children}</p>;
}

export function NavLink({
  href,
  active,
  children,
}: {
  href: string;
  active: boolean;
  children: React.ReactNode;
}) {
  return (
    <Link
      href={href}
      className={`flex items-center gap-3 rounded-md px-2.5 py-2 text-sm transition-colors ${
        active
          ? "bg-border/40 text-fg"
          : "text-muted hover:text-fg hover:bg-border/20"
      }`}
    >
      {children}
    </Link>
  );
}
