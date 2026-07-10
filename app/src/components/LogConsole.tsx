import { useEffect, useRef, useState } from "react";
import { clear, download, type LogEntry, subscribe } from "../ecr17/logger";

function time(ts: number): string {
  return new Date(ts).toLocaleTimeString();
}

export function LogConsole() {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const listRef = useRef<HTMLDivElement>(null);
  const stickRef = useRef(true);

  useEffect(() => subscribe(setEntries), []);

  // Auto-scroll to the newest entry unless the user scrolled up. Re-runs whenever the
  // log grows even though the body only reads refs.
  // biome-ignore lint/correctness/useExhaustiveDependencies: intentionally re-runs on new entries
  useEffect(() => {
    const el = listRef.current;
    if (el && stickRef.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [entries]);

  const onScroll = () => {
    const el = listRef.current;
    if (el) {
      stickRef.current = el.scrollHeight - el.scrollTop - el.clientHeight < 24;
    }
  };

  return (
    <section className="panel" data-testid="log-console">
      <div className="panel__header">
        <h2 className="panel__title">Log</h2>
        <div className="log-actions">
          <button
            type="button"
            className="btn btn--sm"
            onClick={download}
            data-testid="log-download"
          >
            Download
          </button>
          <button type="button" className="btn btn--sm" onClick={clear} data-testid="log-clear">
            Clear
          </button>
        </div>
      </div>
      <div className="log-list" ref={listRef} onScroll={onScroll} data-testid="log-list">
        {entries.length === 0 ? (
          <p className="log-empty">No activity yet.</p>
        ) : (
          entries.map((e) => (
            <div key={e.id} className={`log-line log-line--${e.level}`} data-testid="log-line">
              <span className="log-time">{time(e.ts)}</span>
              <span className={`log-level log-level--${e.level}`}>{e.level}</span>
              <span className="log-label">{e.label}</span>
              {e.detail ? <span className="log-detail">{e.detail}</span> : null}
            </div>
          ))
        )}
      </div>
    </section>
  );
}
