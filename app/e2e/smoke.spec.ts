import { expect, test } from "@playwright/test";

// Scaffold guardrail: proves the Playwright + Vite dev-server pipeline loads the app in a
// real browser. Real UI-interaction scenarios (connect flow, every command, log console,
// busy overlay, config persistence) are authored at MACRO 7 against a mocked Tauri IPC.
test("app loads and shows a heading", async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toBeVisible();
});
