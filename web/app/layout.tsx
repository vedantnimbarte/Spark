import type { Metadata } from "next";
import "./globals.css";
import { Providers } from "./providers";

export const metadata: Metadata = {
  title: "Spark",
  description: "Self-hosted PaaS control plane",
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <head>
        {/*
         * One superfamily, two jobs: Plex Sans carries the interface, Plex Mono
         * is reserved for text the machine produced — commit hashes, durations,
         * byte counts, log output — so monospace stays a signal, not a texture.
         *
         * Linked rather than pulled in through next/font on purpose: next/font
         * downloads the faces during `next build`, which makes the build fail
         * on a machine without access to Google Fonts.
         */}
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link
          rel="preconnect"
          href="https://fonts.gstatic.com"
          crossOrigin="anonymous"
        />
        <link
          href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500&family=IBM+Plex+Sans:wght@400;500;600&display=swap"
          rel="stylesheet"
        />
      </head>
      <body className="bg-base text-fg antialiased">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
