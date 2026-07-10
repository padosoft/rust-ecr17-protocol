import { defineConfig, devices } from "@playwright/test";

// E2E runs against the Vite frontend served by the webServer below (Vite dev on the
// Tauri-fixed port 1420). Today this is just a smoke test; from MACRO 7 the suite mocks
// the Tauri IPC (@tauri-apps/api/mocks `mockIPC`) so every UI interaction is covered
// deterministically without a real POS or the native shell.
export default defineConfig({
  testDir: "./e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 2 : 0,
  reporter: process.env.CI ? [["github"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
  },
});
