"use client";

import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { formatDay, formatDuration, type DayBucket } from "@/lib/api";

/*
 * Recharts draws into SVG attributes, so it needs literal colours rather than
 * the Tailwind classes the rest of the interface uses. These mirror the tokens
 * in globals.css and are the one place they are repeated.
 */
const INK = {
  accent: "#ffb02e",
  // Deliberately duller than the --color-success used for text. Text has to
  // clear a contrast bar; a filled area does not, and the bright green turns a
  // wall of ordinary successes into the loudest thing on the page. Most builds
  // pass — that is the baseline, not the news. The failures are the news.
  successFill: "#35855c",
  danger: "#ff5c5c",
  line: "#1e232d",
  faint: "#5a6273",
} as const;

const AXIS = {
  stroke: INK.faint,
  fontSize: 11,
  fontFamily: "var(--font-mono)",
} as const;

/** Shared dark tooltip; the built-in one is a white card. */
function TooltipCard({
  title,
  rows,
}: {
  title: string;
  rows: { label: string; value: string; colour?: string }[];
}) {
  return (
    <div className="border-line-strong bg-raised rounded-md border px-3 py-2 text-xs shadow-none">
      <p className="text-fg font-medium">{title}</p>
      <div className="mt-1.5 space-y-1">
        {rows.map((row) => (
          <div key={row.label} className="flex items-center gap-2.5">
            {row.colour && (
              <span
                className="size-1.5 shrink-0 rounded-full"
                style={{ background: row.colour }}
                aria-hidden
              />
            )}
            <span className="text-muted">{row.label}</span>
            <span className="tnum text-fg ml-auto font-mono">{row.value}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * Recharts hands the tooltip a readonly array of series entries, each carrying
 * the original row on `payload`. Every chart here has one row per tooltip, so
 * this pulls that row out once instead of casting at each call site.
 */
function hoveredRow<T>(
  active: boolean | undefined,
  payload: readonly { payload?: unknown }[] | undefined,
): T | undefined {
  return active ? (payload?.[0]?.payload as T | undefined) : undefined;
}

/**
 * Deploy activity over the window â€” the page's subject, so it gets the space.
 *
 * Outcomes are stacked rather than grouped: the question a person opens this
 * with is "how much did we ship, and how much of it broke", and a stack answers
 * both in one bar height.
 */
export function DeployActivityChart({ data }: { data: DayBucket[] }) {
  return (
    <div className="h-64 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <BarChart data={data} margin={{ top: 4, right: 4, left: -20, bottom: 0 }}>
          <CartesianGrid stroke={INK.line} vertical={false} />
          <XAxis
            dataKey="day"
            tickFormatter={formatDay}
            tickLine={false}
            axisLine={{ stroke: INK.line }}
            // Thin the labels as the window widens rather than letting 90 of
            // them collide into a grey smear.
            interval={Math.max(0, Math.floor(data.length / 8) - 1)}
            {...AXIS}
          />
          <YAxis
            allowDecimals={false}
            tickLine={false}
            axisLine={false}
            width={44}
            {...AXIS}
          />
          <Tooltip
            cursor={{ fill: "rgba(255,255,255,0.04)" }}
            content={({ active, payload }) => {
              const bucket = hoveredRow<DayBucket>(active, payload);
              if (!bucket) return null;
              return (
                <TooltipCard
                  title={formatDay(bucket.day)}
                  rows={[
                    {
                      label: "Succeeded",
                      value: String(bucket.succeeded),
                      colour: INK.successFill,
                    },
                    {
                      label: "Failed",
                      value: String(bucket.failed),
                      colour: INK.danger,
                    },
                    {
                      label: "Median build",
                      value: formatDuration(bucket.median_build_seconds),
                    },
                  ]}
                />
              );
            }}
          />
          <Bar
            dataKey="succeeded"
            stackId="outcome"
            fill={INK.successFill}
            radius={[0, 0, 0, 0]}
            isAnimationActive={false}
          />
          <Bar
            dataKey="failed"
            stackId="outcome"
            fill={INK.danger}
            radius={[2, 2, 0, 0]}
            isAnimationActive={false}
          />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}

/**
 * Median build time per day. Days where nothing was built are gaps rather than
 * zeroes â€” a zero-second build would be a lie, and `connectNulls` would draw a
 * straight line across a week of silence as though it were data.
 */
export function BuildDurationChart({ data }: { data: DayBucket[] }) {
  return (
    <div className="h-56 w-full">
      <ResponsiveContainer width="100%" height="100%">
        <LineChart data={data} margin={{ top: 8, right: 8, left: 0, bottom: 0 }}>
          <CartesianGrid stroke={INK.line} vertical={false} />
          <XAxis
            dataKey="day"
            tickFormatter={formatDay}
            tickLine={false}
            axisLine={{ stroke: INK.line }}
            interval={Math.max(0, Math.floor(data.length / 10) - 1)}
            {...AXIS}
          />
          <YAxis
            tickFormatter={(value: number) => formatDuration(value)}
            tickLine={false}
            axisLine={false}
            // Wide enough for the longest label the formatter emits ("12m 30s");
            // a narrower gutter clips the leading digits.
            width={64}
            {...AXIS}
          />
          <Tooltip
            cursor={{ stroke: INK.line, strokeWidth: 1 }}
            content={({ active, payload }) => {
              const bucket = hoveredRow<DayBucket>(active, payload);
              if (!bucket) return null;
              return (
                <TooltipCard
                  title={formatDay(bucket.day)}
                  rows={[
                    {
                      label: "Median build",
                      value: formatDuration(bucket.median_build_seconds),
                      colour: INK.accent,
                    },
                    { label: "Deploys", value: String(bucket.total) },
                  ]}
                />
              );
            }}
          />
          <Line
            type="monotone"
            dataKey="median_build_seconds"
            stroke={INK.accent}
            strokeWidth={1.5}
            connectNulls={false}
            dot={{ r: 2, fill: INK.accent, stroke: "none" }}
            activeDot={{ r: 4, fill: INK.accent, stroke: "none" }}
            isAnimationActive={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
}
