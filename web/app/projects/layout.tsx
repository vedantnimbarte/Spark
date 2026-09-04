"use client";

import { useQuery } from "@tanstack/react-query";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { api, ApiError } from "@/lib/api";
import { Sidebar } from "@/components/Sidebar";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const me = useQuery({ queryKey: ["me"], queryFn: () => api.me() });

  // An expired or missing session lands on the login page rather than an
  // endless spinner.
  useEffect(() => {
    if (me.error instanceof ApiError && me.error.status === 401) {
      router.replace("/login");
    }
  }, [me.error, router]);

  if (me.isPending) {
    return (
      <div className="text-faint flex min-h-screen items-center justify-center text-sm">
        Loading…
      </div>
    );
  }

  if (!me.data) return null;

  return (
    <div className="flex min-h-screen">
      <Sidebar email={me.data.email} />
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}
