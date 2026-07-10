import type { Ecr17Config, LrcMode } from "../ecr17/types";

interface Props {
  value: Ecr17Config;
  onChange: (next: Ecr17Config) => void;
}

const LRC_MODES: LrcMode[] = ["stx", "std", "noext", "stx_noext"];

export function ConfigForm({ value, onChange }: Props) {
  const set = <K extends keyof Ecr17Config>(key: K, v: Ecr17Config[K]) =>
    onChange({ ...value, [key]: v });

  const numOrUndef = (s: string): number | undefined => {
    if (s.trim() === "") return undefined;
    const n = Number(s);
    return Number.isFinite(n) ? n : undefined;
  };

  return (
    <section className="panel" data-testid="config-form">
      <h2 className="panel__title">Configuration</h2>
      <div className="form-grid">
        <label className="field">
          <span className="field__label">Host (POS IP)</span>
          <input
            className="input"
            value={value.host}
            onChange={(e) => set("host", e.currentTarget.value)}
            placeholder="192.168.1.50"
            data-testid="cfg-host"
          />
        </label>
        <label className="field">
          <span className="field__label">Port</span>
          <input
            className="input"
            type="number"
            value={value.port ?? ""}
            onChange={(e) => set("port", numOrUndef(e.currentTarget.value))}
            data-testid="cfg-port"
          />
        </label>
        <label className="field">
          <span className="field__label">Terminal ID</span>
          <input
            className="input"
            value={value.terminalId}
            onChange={(e) => set("terminalId", e.currentTarget.value)}
            placeholder="12345678"
            data-testid="cfg-terminalId"
          />
        </label>
        <label className="field">
          <span className="field__label">Cash register ID</span>
          <input
            className="input"
            value={value.cashRegisterId}
            onChange={(e) => set("cashRegisterId", e.currentTarget.value)}
            placeholder="00000001"
            data-testid="cfg-cashRegisterId"
          />
        </label>
        <label className="field">
          <span className="field__label">LRC mode</span>
          <select
            className="input"
            value={value.lrcMode ?? "std"}
            onChange={(e) => set("lrcMode", e.currentTarget.value as LrcMode)}
            data-testid="cfg-lrcMode"
          >
            {LRC_MODES.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
        </label>
        <label className="field">
          <span className="field__label">Response timeout (ms)</span>
          <input
            className="input"
            type="number"
            value={value.responseTimeoutMs ?? ""}
            onChange={(e) => set("responseTimeoutMs", numOrUndef(e.currentTarget.value))}
            data-testid="cfg-responseTimeoutMs"
          />
        </label>
      </div>
      <div className="toggles">
        <label className="toggle">
          <input
            type="checkbox"
            checked={value.keepAlive ?? false}
            onChange={(e) => set("keepAlive", e.currentTarget.checked)}
            data-testid="cfg-keepAlive"
          />
          <span>Keep alive</span>
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={value.autoReconnect ?? false}
            onChange={(e) => set("autoReconnect", e.currentTarget.checked)}
            data-testid="cfg-autoReconnect"
          />
          <span>Auto-reconnect</span>
        </label>
        <label className="toggle">
          <input
            type="checkbox"
            checked={value.debug ?? false}
            onChange={(e) => set("debug", e.currentTarget.checked)}
            data-testid="cfg-debug"
          />
          <span>Debug</span>
        </label>
      </div>
    </section>
  );
}
