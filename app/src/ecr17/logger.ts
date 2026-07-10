// In-memory log store for the ECR17 control panel: a bounded ring buffer with pub/sub for
// the live UI, plus a download-as-file helper. Nothing here ever throws to its callers.

export type LogLevel = "info" | "sent" | "recv" | "progress" | "receipt" | "ok" | "ko" | "error";

export interface LogEntry {
  id: string;
  ts: number;
  level: LogLevel;
  label: string;
  detail?: string;
}

const UI_MAX = 500;

let entries: LogEntry[] = [];
let seq = 0;
const listeners = new Set<(entries: LogEntry[]) => void>();

function emit(): void {
  const snapshot = entries.slice();
  for (const listener of listeners) {
    listener(snapshot);
  }
}

/** Subscribe to the live log. Immediately receives the current snapshot. */
export function subscribe(cb: (entries: LogEntry[]) => void): () => void {
  listeners.add(cb);
  cb(entries.slice());
  return () => {
    listeners.delete(cb);
  };
}

/** Append a log entry. Never throws. */
export function log(level: LogLevel, label: string, detail?: string): void {
  const entry: LogEntry = { id: `${Date.now()}-${seq++}`, ts: Date.now(), level, label, detail };
  entries.push(entry);
  if (entries.length > UI_MAX) {
    entries = entries.slice(-UI_MAX);
  }
  emit();
}

/** Clear the in-memory log. */
export function clear(): void {
  entries = [];
  emit();
}

/** Serializes the current log to a plain-text blob (one ISO line per entry). */
export function toText(): string {
  return entries
    .map(
      (e) =>
        `${new Date(e.ts).toISOString()} [${e.level}] ${e.label}${e.detail ? ` ${e.detail}` : ""}`,
    )
    .join("\n");
}

/** Triggers a browser download of the current log as a .log file. */
export function download(): void {
  try {
    const blob = new Blob([`${toText()}\n`], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = "ecr17-debug.log";
    a.click();
    URL.revokeObjectURL(url);
  } catch {
    // best-effort; ignore
  }
}
