import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

// Scaffold guardrail: proves the Vitest + Testing Library + jsdom pipeline renders a
// React component. Replaced by real component tests starting at MACRO 7.
describe("App (scaffold)", () => {
  it("renders the heading", () => {
    render(<App />);
    expect(screen.getByRole("heading", { level: 1 })).toBeInTheDocument();
  });
});
