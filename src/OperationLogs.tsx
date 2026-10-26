import { memo, useLayoutEffect, useRef, useState } from "react";
import type { OperationLog } from "./types";

// A transfer-rate tick must not rebuild hundreds of unchanged log rows.
export const OperationLogs = memo(function OperationLogs({
  logs, compact = false, limit = 200, emptyMessage = "等待 brew 输出…"
}: {
  logs: OperationLog[];
  compact?: boolean;
  limit?: number;
  emptyMessage?: string;
}) {
  const viewport = useRef<HTMLDivElement>(null);
  const [following, setFollowing] = useState(true);

  useLayoutEffect(() => {
    if (following && viewport.current) viewport.current.scrollTop = viewport.current.scrollHeight;
  }, [logs, following]);

  return (
    <div className="operation-log-region">
      <div className={compact ? "log-box compact" : "log-box"}
        ref={viewport} role="log" aria-label="Homebrew 输出日志" aria-live="off" tabIndex={0}
        onScroll={(event) => {
          const node = event.currentTarget;
          setFollowing(node.scrollHeight - node.clientHeight - node.scrollTop <= 32);
        }}>
        {logs.length ? logs.slice(-limit).map((log, index) => (
          <p key={index} className={log.stream}><b>{log.stream}</b><span>{log.line}</span></p>
        )) : <p className="stdout"><b>system</b><span>{emptyMessage}</span></p>}
      </div>
      {!following && !compact ? (
        <button className="log-follow-button" type="button" onClick={() => setFollowing(true)}>
          回到最新日志 ↓
        </button>
      ) : null}
    </div>
  );
});
