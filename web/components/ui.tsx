"use client";

import Link from "next/link";
import type { Deployment, DeploymentStatus } from "@/lib/api";

/*
 * Three containers, three jobs.
 *
 * Panel   — a discrete object: a project, a chart, a form. Raised off the page.
 * Well    — content the machine wrote: logs, revealed values, inputs. Sunken.
 * Section — a region of the page itself. No box at all, just a heading and a
 *           rule, because most content does not need a box to be understood,
 *           and boxing everything flattens the hierarchy it was meant to make.
 */

export function Panel({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`border-line bg-raised rounded-lg border ${className}`}>
      {children}
    </div>
  );
}

export function Well({
  children,
  className = "",
}: {
  children: React.ReactNode;
  className?: string;
}) {
  return (
    <div className={`border-line bg-sunken rounded-md border ${className}`}>
      {children}
    </div>
  );
}

export function Section({
  title,
  description,
  actions,
  children,
}: {
  title: string;
  description?: string;
  actions?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="mt-10">
      <div className="border-line flex flex-wrap items-end justify-between gap-3 border-b pb-2.5">
        <div>
          <h2 className="text-[13px] font-semibold tracking-tight">{title}</h2>
          {description && (
            <p className="text-faint mt-0.5 text-xs">{description}</p>
          )}
        </div>
        {actions}
      </div>
      <div className="mt-4">{children}</div>
    </section>
  );
}

/** Page title block. One per page, always the first thing in the content area. */
export function PageHeader({
  title,
  description,
  actions,
}: {
  title: string;
  description?: React.ReactNode;
  actions?: React.ReactNode;
}) {
  return (
    <header className="flex flex-wrap items-start justify-between gap-4">
      <div className="min-w-0">
        <h1 className="text-[22px] leading-tight font-semibold tracking-tight">
          {title}
        </h1>
        {description && (
          <div className="text-muted mt-1 text-[13px]">{description}</div>
        )}
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </header>
  );
}

type ButtonVariant = "primary" | "secondary" | "danger" | "ghost";

const BUTTON_STYLES: Record<ButtonVariant, string> = {
  // Amber is the "in flight" colour, and every primary action here starts
  // something: deploy, create, sign in.
  primary:
    "bg-accent text-[#17130a] border-transparent hover:bg-accent/90 font-semibold",
  secondary:
    "bg-raised text-fg border-line hover:border-line-strong hover:bg-line/40",
  danger:
    "bg-transparent text-danger border-danger/35 hover:border-danger hover:bg-danger/10",
  ghost: "bg-transparent text-muted border-transparent hover:text-fg",
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
      className={`inline-flex items-center justify-center gap-1.5 rounded-md border px-3 py-1.5 text-[13px] font-medium transition-colors disabled:cursor-not-allowed disabled:opacity-40 ${BUTTON_STYLES[variant]} ${className}`}
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
      className={`border-line bg-sunken text-fg placeholder:text-faint focus:border-line-strong w-full rounded-md border px-3 py-2 text-[13px] transition-colors focus:outline-none ${className}`}
    />
  );
}

/**
 * A labelled control. The label is sentence case at a small size: uppercase
 * tracking costs legibility and says nothing that the size and colour were not
 * already saying.
 */
export function Field({
  label,
  hint,
  htmlFor,
  children,
}: {
  label: string;
  hint?: React.ReactNode;
  htmlFor?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <label htmlFor={htmlFor} className="text-muted block text-xs font-medium">
        {label}
      </label>
      {children}
      {hint && <p className="text-faint text-xs">{hint}</p>}
    </div>
  );
}

/** The bare label, for the forms that lay out their own fields. */
export function Label({ children }: { children: React.ReactNode }) {
  return (
    <span className="text-muted block text-xs font-medium">{children}</span>
  );
}

/** A read-only label and value, for facts rather than inputs. */
export function Detail({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="min-w-0">
      <p className="text-faint text-xs">{label}</p>
      <div className="mt-1 text-[13px]">{children}</div>
    </div>
  );
}

/*
 * Deployment status is a lifecycle, not five unrelated tags: queued, then two
 * working states, then one of two outcomes. The colours follow that shape —
 * cool while waiting, the accent while working, resolved at the end.
 */
const STATUS: Record<
  DeploymentStatus,
  { dot: string; text: string; tint: string; label: string }
> = {
  pending: {
    dot: "bg-queued",
    text: "text-queued",
    tint: "bg-queued/10 border-queued/25",
    label: "Queued",
  },
  building: {
    dot: "bg-accent",
    text: "text-accent",
    tint: "bg-accent/10 border-accent/25",
    label: "Building",
  },
  deploying: {
    dot: "bg-accent",
    text: "text-accent",
    tint: "bg-accent/10 border-accent/25",
    label: "Deploying",
  },
  deployed: {
    dot: "bg-success",
    text: "text-success",
    tint: "bg-success/10 border-success/25",
    label: "Deployed",
  },
  failed: {
    dot: "bg-danger",
    text: "text-danger",
    tint: "bg-danger/10 border-danger/25",
    label: "Failed",
  },
};

export function isInFlight(status: DeploymentStatus): boolean {
  return (
    status === "pending" || status === "building" || status === "deploying"
  );
}

export function StatusBadge({
  status,
  showLabel = true,
}: {
  status: DeploymentStatus;
  showLabel?: boolean;
}) {
  const it = STATUS[status];
  return (
    <span className="inline-flex items-center gap-1.5 text-xs">
      <span
        className={`size-1.5 shrink-0 rounded-full ${it.dot} ${
          isInFlight(status) ? "animate-pulse-dot" : ""
        }`}
        aria-hidden
      />
      {showLabel ? (
        <span className={it.text}>{it.label}</span>
      ) : (
        <span className="sr-only">{it.label}</span>
      )}
    </span>
  );
}

/** The prominent form, for a header where the state is the headline. */
export function StatusPill({ status }: { status: DeploymentStatus }) {
  const it = STATUS[status];
  return (
    <span
      className={`inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-medium ${it.tint} ${it.text}`}
    >
      <span
        className={`size-1.5 rounded-full ${it.dot} ${
          isInFlight(status) ? "animate-pulse-dot" : ""
        }`}
        aria-hidden
      />
      {it.label}
    </span>
  );
}

/**
 * The one piece of ambient motion in the product: a travelling hairline that
 * marks work whose duration cannot be known in advance. It appears only while
 * a build is actually running.
 */
export function ActivityLine() {
  return (
    <div className="bg-line/60 h-px w-full overflow-hidden" aria-hidden>
      <div className="animate-sweep bg-accent h-px w-1/4" />
    </div>
  );
}

/**
 * A row of figures separated by hairlines rather than boxed individually. Four
 * bordered cards read as four unrelated objects; this is one object with four
 * readings.
 */
export function StatRow({ children }: { children: React.ReactNode }) {
  return (
    <Panel className="divide-line grid divide-y sm:grid-cols-2 lg:grid-cols-4">
      {children}
    </Panel>
  );
}

export function Stat({
  label,
  value,
  detail,
  tone = "text-fg",
}: {
  label: string;
  value: React.ReactNode;
  detail?: React.ReactNode;
  tone?: string;
}) {
  return (
    <div className="border-line px-5 py-4 sm:[&:not(:nth-child(odd))]:border-l lg:[&:not(:first-child)]:border-l">
      <p className="text-faint text-xs">{label}</p>
      <p
        className={`tnum mt-1.5 text-[26px] leading-none font-semibold tracking-tight ${tone}`}
      >
        {value}
      </p>
      {detail && <p className="text-faint mt-1.5 text-xs">{detail}</p>}
    </div>
  );
}

/** Mutually exclusive options; used for the analytics time window. */
export function SegmentedControl<T extends string | number>({
  options,
  value,
  onChange,
  label,
}: {
  options: { value: T; label: string }[];
  value: T;
  onChange: (value: T) => void;
  label: string;
}) {
  return (
    <div
      role="group"
      aria-label={label}
      className="border-line bg-raised inline-flex rounded-md border p-0.5"
    >
      {options.map((option) => (
        <button
          key={String(option.value)}
          type="button"
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
          className={`rounded px-2.5 py-1 text-xs font-medium transition-colors ${
            option.value === value ? "bg-line text-fg" : "text-muted hover:text-fg"
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function Empty({
  title,
  hint,
  action,
}: {
  title: string;
  hint?: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <div className="border-line rounded-lg border border-dashed px-6 py-14 text-center">
      <p className="text-[13px] font-medium">{title}</p>
      {hint && (
        <div className="text-faint mx-auto mt-1.5 max-w-sm text-xs">{hint}</div>
      )}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}

export function ErrorText({ children }: { children: React.ReactNode }) {
  return (
    <p role="alert" className="text-danger text-[13px]">
      {children}
    </p>
  );
}

/** A settled, non-alarming confirmation. */
export function SuccessText({ children }: { children: React.ReactNode }) {
  return (
    <p role="status" className="text-success text-[13px]">
      {children}
    </p>
  );
}

/**
 * Occupies the space the content will occupy, so a panel does not jump when
 * the query lands.
 */
export function Skeleton({ className = "" }: { className?: string }) {
  return (
    <div className={`bg-line/50 animate-pulse-dot rounded ${className}`} aria-hidden />
  );
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
      aria-current={active ? "page" : undefined}
      className={`relative flex items-center gap-2.5 rounded-md px-2.5 py-2 text-[13px] transition-colors ${
        active
          ? "bg-line/60 text-fg font-medium"
          : "text-muted hover:text-fg hover:bg-line/30"
      }`}
    >
      {/* The active marker is a bar at the rail, not colour alone. */}
      {active && (
        <span
          className="bg-accent absolute top-1.5 bottom-1.5 -left-2 w-0.5 rounded-full"
          aria-hidden
        />
      )}
      {children}
    </Link>
  );
}

/**
 * The last handful of deployments as coloured ticks, oldest to newest — enough
 * to read a project's recent run of luck without opening it.
 */
export function OutcomeSparkline({
  deployments,
  limit = 12,
}: {
  deployments: Deployment[];
  limit?: number;
}) {
  const recent = deployments.slice(0, limit).reverse();

  if (recent.length === 0) {
    return <div className="h-5" aria-hidden />;
  }

  return (
    <div
      className="flex h-5 items-end gap-[3px]"
      role="img"
      aria-label={`Last ${recent.length} deployments, oldest first`}
    >
      {recent.map((deployment) => {
        const failed = deployment.status === "failed";
        const settled = failed || deployment.status === "deployed";
        return (
          <span
            key={deployment.id}
            title={`${deployment.status} · ${new Date(
              deployment.created_at,
            ).toLocaleString()}`}
            className={`w-[3px] rounded-full ${
              failed
                ? "bg-danger h-5"
                : settled
                  ? "bg-success/45 h-3"
                  : "bg-accent animate-pulse-dot h-4"
            }`}
          />
        );
      })}
    </div>
  );
}
