import { useCallback, useEffect, useRef, useState } from "react";
import { BusyOverlay } from "./components/BusyOverlay";
import { CommandPalette } from "./components/CommandPalette";
import { CommandParamsSheet } from "./components/CommandParamsSheet";
import { ConfigForm } from "./components/ConfigForm";
import { ConnectionBar } from "./components/ConnectionBar";
import { LogConsole } from "./components/LogConsole";
import type { CommandDef } from "./ecr17/commands";
import { loadConfig, saveConfig } from "./ecr17/storage";
import type { Ecr17Config } from "./ecr17/types";
import { type RunResult, useEcr17 } from "./ecr17/useEcr17";
import "./App.css";

interface Toast {
  text: string;
  tone: "ok" | "ko" | "error";
}

function App() {
  const [config, setConfig] = useState<Ecr17Config>(() => loadConfig());
  const [sheetCmd, setSheetCmd] = useState<CommandDef | null>(null);
  const [toast, setToast] = useState<Toast | null>(null);

  const { connectionState, busy, lastProgress, connect, disconnect, run } = useEcr17(config);

  // Track the pending auto-hide so rapid successive runs don't let an older timer clear a
  // newer toast prematurely.
  const toastTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  useEffect(
    () => () => {
      if (toastTimer.current) {
        clearTimeout(toastTimer.current);
      }
    },
    [],
  );

  const onChangeConfig = useCallback((next: Ecr17Config) => {
    setConfig(next);
    saveConfig(next);
  }, []);

  const showToast = useCallback((r: RunResult) => {
    if (r.status === "error") {
      setToast({ text: "Error — see log", tone: "error" });
    } else if (r.status === "ko") {
      const res = r.result as { outcome?: string; responseId?: string } | undefined;
      const label = res?.outcome ?? (res?.responseId != null ? `vas ${res.responseId}` : "ko");
      setToast({ text: `KO (${label})`, tone: "ko" });
    } else {
      setToast({ text: "OK", tone: "ok" });
    }
    if (toastTimer.current) {
      clearTimeout(toastTimer.current);
    }
    toastTimer.current = setTimeout(() => setToast(null), 2500);
  }, []);

  const doRun = useCallback(
    async (key: string, params: Record<string, unknown>) => {
      const result = await run(key, params);
      showToast(result);
    },
    [run, showToast],
  );

  const onPick = useCallback(
    (cmd: CommandDef) => {
      if (cmd.fields.length === 0) {
        void doRun(cmd.key, {});
      } else {
        setSheetCmd(cmd);
      }
    },
    [doRun],
  );

  return (
    <main className="app" data-testid="app">
      <header className="app__header">
        <h1 className="app__title">
          <span className="app__logo" aria-hidden="true">
            💳
          </span>
          ECR17 Control Panel
        </h1>
        <ConnectionBar
          state={connectionState}
          busy={busy}
          onConnect={() => void connect()}
          onDisconnect={() => void disconnect()}
        />
      </header>

      <div className="app__body">
        <div className="app__col">
          <ConfigForm value={config} onChange={onChangeConfig} />
          <CommandPalette disabled={busy} onPick={onPick} />
        </div>
        <div className="app__col app__col--log">
          <LogConsole />
        </div>
      </div>

      <CommandParamsSheet
        command={sheetCmd}
        onClose={() => setSheetCmd(null)}
        onSubmit={(key, params) => void doRun(key, params)}
      />
      <BusyOverlay visible={busy} progress={lastProgress} />

      {toast ? (
        <div className={`toast toast--${toast.tone}`} data-testid="toast" role="status">
          {toast.text}
        </div>
      ) : null}
    </main>
  );
}

export default App;
