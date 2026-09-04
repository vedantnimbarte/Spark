"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { useState } from "react";

export function Providers({ children }: { children: React.ReactNode }) {
  // Created once per browser session rather than at module scope, so a server
  // render never shares a cache between users.
  const [client] = useState(
    () =>
      new QueryClient({
        defaultOptions: {
          queries: {
            staleTime: 5_000,
            // A 401 means the session is gone; retrying cannot help.
            retry: (failureCount, error) =>
              !(error instanceof Error && error.name === "ApiError") &&
              failureCount < 2,
            refetchOnWindowFocus: false,
          },
        },
      }),
  );

  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
