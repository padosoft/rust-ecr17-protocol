import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Vitest runs the frontend unit tests (components, hooks, pure logic) in jsdom.
// Kept separate from vite.config.ts so the Tauri dev-server tweaks don't affect tests.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test/setup.ts"],
    // Playwright specs live in e2e/ and must not be picked up by Vitest.
    exclude: ["e2e/**", "node_modules/**", "dist/**"],
    css: false,
  },
});
