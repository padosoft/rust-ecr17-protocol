// A minimal, self-contained mock of the Tauri v2 IPC internals, installed into the page
// (via addInitScript) BEFORE the app loads. It implements exactly what @tauri-apps/api uses:
// `window.__TAURI_INTERNALS__.{invoke, transformCallback, unregisterCallback}` plus the
// `plugin:event|listen`/`unlisten` commands, and exposes `window.__ecr17mock` so tests can
// script command responses and emit backend events. Must be a standalone function (no
// imports) so Playwright can serialize it into the page.

export function installTauriMock(): void {
  const w = window as unknown as {
    __TAURI_INTERNALS__: unknown;
    __ecr17mock: unknown;
  };
  const callbacks = new Map<number, (arg: unknown) => void>();
  const listeners = new Map<string, Set<number>>();
  const responses = new Map<string, unknown>();
  let nextId = 1;

  // Backend commands that return `Result<(), String>` (no payload). Any OTHER command that a
  // test invokes without a configured response is a mistake (typo or forgotten setResponse) —
  // we throw so the contract drift fails the test loudly instead of resolving a silent null.
  const VOID_COMMANDS = new Set([
    "configure",
    "connect",
    "disconnect",
    "enable_ecr_printing",
    "reprint",
  ]);

  w.__TAURI_INTERNALS__ = {
    transformCallback(cb: (arg: unknown) => void) {
      const id = nextId++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback(id: number) {
      callbacks.delete(id);
    },
    async invoke(cmd: string, args: Record<string, unknown>) {
      if (cmd === "plugin:event|listen") {
        const event = args.event as string;
        const handler = args.handler as number;
        const set = listeners.get(event) ?? new Set<number>();
        set.add(handler);
        listeners.set(event, set);
        return nextId++;
      }
      if (cmd === "plugin:event|unlisten") {
        return null;
      }
      if (!responses.has(cmd)) {
        if (VOID_COMMANDS.has(cmd)) {
          return null; // known no-payload commands resolve successfully
        }
        throw new Error(`Unmocked command "${cmd}" — call setResponse/setError first`);
      }
      const r = responses.get(cmd) as {
        __error?: boolean;
        message?: string;
        __delayMs?: number;
        value?: unknown;
      };
      if (r && typeof r === "object" && r.__error) {
        throw new Error(r.message ?? "error");
      }
      if (r && typeof r === "object" && typeof r.__delayMs === "number") {
        await new Promise((res) => setTimeout(res, r.__delayMs));
        return r.value ?? null;
      }
      return r;
    },
  };

  w.__ecr17mock = {
    setResponse(cmd: string, value: unknown) {
      responses.set(cmd, value);
    },
    setError(cmd: string, message: string) {
      responses.set(cmd, { __error: true, message });
    },
    setDelayed(cmd: string, value: unknown, ms: number) {
      responses.set(cmd, { __delayMs: ms, value });
    },
    reset() {
      responses.clear();
    },
    emit(event: string, payload: unknown) {
      const set = listeners.get(event);
      if (!set) {
        return;
      }
      for (const id of set) {
        const cb = callbacks.get(id);
        if (cb) {
          cb({ event, id, payload });
        }
      }
    },
  };
}
