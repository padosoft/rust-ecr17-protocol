import { defineConfig, devices } from "@playwright/test";

// E2E runs against the Vite frontend with the Tauri IPC mocked (@tauri-apps/api/mocks),
// so every UI interaction is covered deterministically without a real POS or the native
// shell. The webServer boots the Vite dev server on the Tauri-fixed port 1420.
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
