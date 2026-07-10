interface Props {
  visible: boolean;
  progress: string;
}

export function BusyOverlay({ visible, progress }: Props) {
  if (!visible) {
    return null;
  }
  return (
    <div className="overlay" data-testid="busy-overlay" role="status" aria-live="polite">
      <div className="overlay__card">
        <span className="spinner" aria-hidden="true" />
        <span className="overlay__text">{progress || "Working…"}</span>
      </div>
    </div>
  );
}
