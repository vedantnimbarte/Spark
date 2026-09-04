"use client";

import { useMutation, useQueryClient } from "@tanstack/react-query";
import { usePathname, useRouter } from "next/navigation";
import { useState } from "react";
import { api } from "@/lib/api";
import {
  ChartIcon,
  ChevronIcon,
  GearIcon,
  GridIcon,
  SparkIcon,
} from "./icons";
import { NavLink } from "./ui";

const NAV = [
  { href: "/projects", label: "Projects", Icon: GridIcon },
  { href: "/analytics", label: "Analytics", Icon: ChartIcon },
  { href: "/settings", label: "Settings", Icon: GearIcon },
];

export function Sidebar({ email }: { email: string }) {
  const [collapsed, setCollapsed] = useState(false);
  const pathname = usePathname();
  const router = useRouter();
  const queryClient = useQueryClient();

  const logout = useMutation({
    mutationFn: () => api.logout(),
    onSuccess: () => {
      queryClient.clear();
      router.push("/login");
    },
  });

  return (
    <aside
      className={`border-border flex shrink-0 flex-col border-r transition-[width] ${
        collapsed ? "w-14" : "w-56"
      }`}
    >
      <div className="border-border flex h-14 items-center gap-2 border-b px-4">
        <SparkIcon className="text-accent size-4 shrink-0" />
        {!collapsed && (
          <span className="text-sm font-semibold tracking-tight">Spark</span>
        )}
      </div>

      <nav className="flex-1 space-y-0.5 p-2">
        {NAV.map(({ href, label, Icon }) => (
          <NavLink
            key={href}
            href={href}
            active={pathname.startsWith(href)}
          >
            <Icon className="size-4 shrink-0" />
            {!collapsed && label}
          </NavLink>
        ))}
      </nav>

      <div className="border-border space-y-2 border-t p-2">
        {!collapsed && (
          <p className="text-faint truncate px-2.5 text-xs" title={email}>
            {email}
          </p>
        )}
        <button
          onClick={() => logout.mutate()}
          className="text-muted hover:text-fg w-full rounded-md px-2.5 py-1.5 text-left text-xs transition-colors"
        >
          {collapsed ? "→" : "Sign out"}
        </button>
        <button
          onClick={() => setCollapsed((c) => !c)}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          className="text-faint hover:text-fg w-full rounded-md px-2.5 py-1.5 text-left transition-colors"
        >
          <ChevronIcon
            className={`size-4 transition-transform ${collapsed ? "" : "rotate-180"}`}
          />
        </button>
      </div>
    </aside>
  );
}
