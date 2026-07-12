import type { ConnectionState } from "../ecr17/types";

interface Props {
  state: ConnectionState;
  busy: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
}

const LABEL: Record<ConnectionState, string> = {
  disconnected: "Disconnected",
  connecting: "Connecting…",
  connected: "Connected",
};

export function ConnectionBar({ state, busy, onConnect, onDisconnect }: Props) {
  const connected = state === "connected";
  return (
    <div className="conn-bar" data-testid="connection-bar">
      <span className={`conn-dot conn-dot--${state}`} aria-hidden="true" />
      <span className="conn-label" data-testid="connection-state">
        {LABEL[state]}
      </span>
      <div className="conn-actions">
        <button
          type="button"
          className="btn btn--primary"
          onClick={onConnect}
          disabled={busy || connected}
          data-testid="btn-connect"
        >
          Connect
        </button>
        <button
          type="button"
          className="btn"
          onClick={onDisconnect}
          disabled={busy || state === "disconnected"}
          data-testid="btn-disconnect"
        >
          Disconnect
        </button>
      </div>
    </div>
  );
}
