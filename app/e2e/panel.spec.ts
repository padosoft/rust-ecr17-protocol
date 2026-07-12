import { expect, type Page, test } from "@playwright/test";
import { installTauriMock } from "./tauri-mock";

// Every test installs the Tauri IPC mock before the app loads, then drives the real UI.
test.beforeEach(async ({ page }) => {
  await page.addInitScript(installTauriMock);
  await page.goto("/");
  await expect(page.getByTestId("app")).toBeVisible();
});

type Mock = {
  setResponse: (cmd: string, value: unknown) => void;
  setError: (cmd: string, message: string) => void;
  setDelayed: (cmd: string, value: unknown, ms: number) => void;
  emit: (event: string, payload: unknown) => void;
};

const setResponse = (page: Page, cmd: string, value: unknown) =>
  page.evaluate(
    ([c, v]) => (window as unknown as { __ecr17mock: Mock }).__ecr17mock.setResponse(c, v),
    [cmd, value] as const,
  );
const setError = (page: Page, cmd: string, message: string) =>
  page.evaluate(
    ([c, m]) => (window as unknown as { __ecr17mock: Mock }).__ecr17mock.setError(c, m),
    [cmd, message] as const,
  );
const setDelayed = (page: Page, cmd: string, value: unknown, ms: number) =>
  page.evaluate(
    ([c, v, m]) =>
      (window as unknown as { __ecr17mock: Mock }).__ecr17mock.setDelayed(c, v, m as number),
    [cmd, value, ms] as const,
  );
const emit = (page: Page, event: string, payload: unknown) =>
  page.evaluate(([e, p]) => (window as unknown as { __ecr17mock: Mock }).__ecr17mock.emit(e, p), [
    event,
    payload,
  ] as const);

test("loads the control panel disconnected", async ({ page }) => {
  await expect(page.getByRole("heading", { level: 1 })).toHaveText(/ECR17 Control Panel/);
  await expect(page.getByTestId("connection-state")).toHaveText("Disconnected");
  await expect(page.getByTestId("command-palette")).toBeVisible();
});

test("guards Connect when the host is empty", async ({ page }) => {
  await page.getByTestId("cfg-host").fill("");
  await page.getByTestId("btn-connect").click();
  await expect(page.getByTestId("log-list")).toContainText("Host is empty");
});

test("connect flow updates state and log on the backend event", async ({ page }) => {
  await page.getByTestId("cfg-host").fill("10.0.0.5");
  await page.getByTestId("btn-connect").click();
  await emit(page, "ecr17:connection", "connected");
  await expect(page.getByTestId("connection-state")).toHaveText("Connected");
  await expect(page.getByTestId("log-list")).toContainText("connected");
});

test("runs a no-argument command and shows an OK toast", async ({ page }) => {
  await setResponse(page, "status", {
    terminalId: "12345678",
    terminalDateTime: "2025-02-01T15:30:00",
    status: 2,
    softwareRelease: "V1.2.3",
  });
  await page.getByTestId("cmd-status").click();
  await expect(page.getByTestId("toast")).toHaveText("OK");
  await expect(page.getByTestId("log-list")).toContainText("status →");
});

test("opens the params sheet for a command with fields and validates required inputs", async ({
  page,
}) => {
  await page.getByTestId("cmd-pay").click();
  await expect(page.getByTestId("params-sheet")).toBeVisible();
  // Amount is required → submit disabled until filled.
  await expect(page.getByTestId("sheet-submit")).toBeDisabled();
  await page.getByTestId("input-amountCents").fill("6.50");
  await expect(page.getByTestId("sheet-submit")).toBeEnabled();
});

test("submits a payment and masks the PAN in the log", async ({ page }) => {
  await setResponse(page, "pay", {
    outcome: "ok",
    resultCode: "00",
    pan: "4111111111111111",
    authCode: "AUTH01",
  });
  await page.getByTestId("cmd-pay").click();
  await page.getByTestId("input-amountCents").fill("6.50");
  await page.getByTestId("sheet-submit").click();

  await expect(page.getByTestId("toast")).toHaveText("OK");
  const log = page.getByTestId("log-list");
  await expect(log).toContainText("************1111"); // masked PAN
  await expect(log).not.toContainText("4111111111111111"); // never the raw PAN
});

test("shows a KO toast for a declined result", async ({ page }) => {
  await setResponse(page, "totals", { outcome: "ko", resultCode: "01", posTotalCents: 0 });
  await page.getByTestId("cmd-totals").click();
  await expect(page.getByTestId("toast")).toHaveText("KO (ko)");
});

test("shows an Error toast when a command rejects", async ({ page }) => {
  await setError(page, "status", "boom");
  await page.getByTestId("cmd-status").click();
  await expect(page.getByTestId("toast")).toHaveText("Error — see log");
  await expect(page.getByTestId("log-list")).toContainText("status failed");
});

test("shows the busy overlay while a command is in flight", async ({ page }) => {
  await setDelayed(
    page,
    "status",
    { terminalId: "1", terminalDateTime: "", status: 0, softwareRelease: "" },
    700,
  );
  await page.getByTestId("cmd-status").click();
  await expect(page.getByTestId("busy-overlay")).toBeVisible();
  await expect(page.getByTestId("busy-overlay")).toBeHidden();
});

test("forwards progress and receipt events to the log", async ({ page }) => {
  await emit(page, "ecr17:progress", { message: "ATTENDERE PREGO" });
  await emit(page, "ecr17:receipt", { text: "LINE 1" });
  const log = page.getByTestId("log-list");
  await expect(log).toContainText("ATTENDERE PREGO");
  await expect(log).toContainText("LINE 1");
});

test("clears the log", async ({ page }) => {
  await emit(page, "ecr17:progress", { message: "SOMETHING" });
  await expect(page.getByTestId("log-list")).toContainText("SOMETHING");
  await page.getByTestId("log-clear").click();
  await expect(page.getByTestId("log-list")).toContainText("No activity yet");
});

test("persists the config across a reload", async ({ page }) => {
  await page.getByTestId("cfg-host").fill("192.168.7.7");
  await page.getByTestId("cfg-terminalId").fill("87654321");
  await page.reload();
  await expect(page.getByTestId("cfg-host")).toHaveValue("192.168.7.7");
  await expect(page.getByTestId("cfg-terminalId")).toHaveValue("87654321");
});
