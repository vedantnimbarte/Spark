import type { NextConfig } from "next";

const API_ORIGIN = process.env.SPARK_API_ORIGIN ?? "http://localhost:8080";

const config: NextConfig = {
  // Standalone output is only for the container image. It is opt-in because
  // emitting it requires symlinks, which Windows refuses without Developer
  // Mode, so an unconditional setting breaks `pnpm build` on a Windows host.
  ...(process.env.NEXT_STANDALONE === "1" ? { output: "standalone" as const } : {}),
  // Proxying keeps the dashboard and API on one origin, so the session cookie
  // is same-origin and there is no CORS or SameSite handling to get wrong.
  async rewrites() {
    return [{ source: "/api/:path*", destination: `${API_ORIGIN}/api/:path*` }];
  },
};

export default config;
