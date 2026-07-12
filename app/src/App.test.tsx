import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

// The Tauri event API isn't available under jsdom; stub it so mounting useEcr17's listeners
// doesn't reject noisily. Command invocation is exercised end-to-end by the Playwright suite.
vi.mock("@tauri-apps/api/event", () => ({
  listen: () => Promise.resolve(() => {}),
}));
vi.mock("@tauri-apps/api/core", () => ({
  invoke: () => Promise.resolve(null),
}));

describe("App", () => {
  it("renders the control panel shell", () => {
    render(<App />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("ECR17 Control Panel");
    expect(screen.getByTestId("connection-state")).toHaveTextContent("Disconnected");
    // The command palette exposes every command; check a couple.
    expect(screen.getByTestId("cmd-status")).toBeInTheDocument();
    expect(screen.getByTestId("cmd-pay")).toBeInTheDocument();
    expect(screen.getByTestId("config-form")).toBeInTheDocument();
  });
});
