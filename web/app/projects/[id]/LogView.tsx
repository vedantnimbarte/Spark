"use client";

import { useEffect, useRef, useState } from "react";

/**
 * Follows a deployment's build log over server-sent events.
 *
 * EventSource is used rather than a polling fetch because it reconnects on its
 * own and the endpoint is one-directional.
 */
export function LogView({
  appId,
  deploymentId,
}: {
  appId: string;
  deploymentId: string;
}) {
  const [lines, setLines] = useState<string[]>([]);
  const [done, setDone] = useState(false);
  const bottom = useRef<HTMLDivElement>(null);
  const pinned = useRef(true);

  useEffect(() => {
    setLines([]);
    setDone(false);

    const source = new EventSource(
      `/api/v1/deployments/${deploymentId}/logs`,
    );

    source.onmessage = (event) => {
      setLines((current) => [...current, event.data as string]);
    };

    // The server emits a named `end` event when the deployment reaches a
    // terminal state, so the stream closes instead of reconnecting forever.
    source.addEventListener("end", () => {
      setDone(true);
      source.close();
    });

    source.onerror = () => {
      // EventSource retries by itself; only a closed connection is terminal.
      if (source.readyState === EventSource.CLOSED) setDone(true);
    };

    return () => source.close();
  }, [deploymentId, appId]);

  // Follow the tail, but stop fighting the user if they scroll up to read.
  useEffect(() => {
    if (pinned.current) {
      bottom.current?.scrollIntoView({ block: "end" });
    }
  }, [lines]);

  return (
    <div className="flex h-[28rem] flex-col">
      <div className="border-border flex items-center justify-between border-b px-4 py-2">
        <span className="text-muted text-xs font-medium">Build log</span>
        <span className="text-faint text-xs">
          {done ? "finished" : "streaming…"}
        </span>
      </div>

      <div
        onScroll={(e) => {
          const el = e.currentTarget;
          pinned.current =
            el.scrollHeight - el.scrollTop - el.clientHeight < 40;
        }}
        className="flex-1 overflow-auto px-4 py-3"
      >
        {lines.length === 0 ? (
          <p className="text-faint font-mono text-xs">
            {done ? "No output." : "Waiting for output…"}
          </p>
        ) : (
          <pre className="text-fg/80 font-mono text-xs leading-relaxed whitespace-pre-wrap">
            {lines.join("\n")}
          </pre>
        )}
        <div ref={bottom} />
      </div>
    </div>
  );
}
