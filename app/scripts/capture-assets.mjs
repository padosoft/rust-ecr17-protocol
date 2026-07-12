// Generates the README banner and control-panel screenshots with the bundled Chromium.
// Run from app/ with the Vite dev server up on :1420:
//   node scripts/capture-assets.mjs
// Output lands in ../resources/ (banner.png) and ../resources/screenshots/*.png.

import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "@playwright/test";

const here = dirname(fileURLToPath(import.meta.url));
const resources = resolve(here, "../../resources");
const shots = resolve(resources, "screenshots");
mkdirSync(shots, { recursive: true });

// --- The same Tauri IPC mock the e2e suite installs, inlined so this script is standalone.
const MOCK = () => {
  const w = window;
  const callbacks = new Map();
  const listeners = new Map();
  const responses = new Map();
  let nextId = 1;
  const VOID = new Set(["configure", "connect", "disconnect", "enable_ecr_printing", "reprint"]);
  w.__TAURI_INTERNALS__ = {
    transformCallback(cb) {
      const id = nextId++;
      callbacks.set(id, cb);
      return id;
    },
    unregisterCallback(id) {
      callbacks.delete(id);
    },
    async invoke(cmd, args) {
      if (cmd === "plugin:event|listen") {
        const set = listeners.get(args.event) ?? new Set();
        set.add(args.handler);
        listeners.set(args.event, set);
        return nextId++;
      }
      if (cmd === "plugin:event|unlisten") return null;
      if (!responses.has(cmd)) {
        if (VOID.has(cmd)) return null;
        throw new Error(`Unmocked command "${cmd}"`);
      }
      return responses.get(cmd);
    },
  };
  w.__ecr17mock = {
    setResponse: (cmd, value) => responses.set(cmd, value),
    emit(event, payload) {
      const set = listeners.get(event);
      if (!set) return;
      for (const id of set) callbacks.get(id)?.({ event, id, payload });
    },
  };
};

const browser = await chromium.launch();

// --- 1) Banner --------------------------------------------------------------
{
  const page = await browser.newPage({ viewport: { width: 1280, height: 440 }, deviceScaleFactor: 2 });
  await page.setContent(`<!doctype html><html><head><meta charset="utf-8"><style>
    * { margin: 0; box-sizing: border-box; }
    body { width: 1280px; height: 440px; font-family: -apple-system, "Segoe UI", Roboto, sans-serif;
      background: radial-gradient(1200px 500px at 78% -12%, #1e3a5f 0%, transparent 60%),
                  radial-gradient(900px 500px at 12% 120%, #3b1e5f 0%, transparent 55%), #0b0f19;
      color: #e8edf6; display: flex; flex-direction: column; justify-content: center;
      padding: 0 84px; position: relative; overflow: hidden; }
    .grid { position: absolute; inset: 0; background-image:
      linear-gradient(#ffffff0a 1px, transparent 1px), linear-gradient(90deg, #ffffff0a 1px, transparent 1px);
      background-size: 40px 40px; mask-image: radial-gradient(900px 420px at 70% 30%, #000 30%, transparent 75%); }
    .chip { display: inline-flex; align-items: center; gap: 10px; font-size: 20px; font-weight: 600;
      color: #7dd3fc; letter-spacing: .12em; text-transform: uppercase; margin-bottom: 22px; }
    h1 { font-size: 76px; font-weight: 800; letter-spacing: -.02em; line-height: 1;
      background: linear-gradient(92deg, #fff 20%, #9fd0ff 70%, #c9a2ff); -webkit-background-clip: text;
      background-clip: text; color: transparent; }
    h1 .mono { font-family: "SFMono-Regular", ui-monospace, "Cascadia Code", monospace; }
    p { font-size: 27px; color: #aab8cf; margin-top: 22px; max-width: 820px; line-height: 1.42; }
    .tags { display: flex; gap: 12px; margin-top: 30px; }
    .tag { font-size: 17px; font-weight: 600; padding: 9px 18px; border-radius: 999px;
      border: 1px solid #ffffff1f; background: #ffffff0d; color: #cdd8ea; }
    .coin { position: absolute; right: 60px; top: 50%; transform: translateY(-50%); font-size: 168px;
      filter: drop-shadow(0 22px 60px #0af5) drop-shadow(0 0 42px #38bdf870); }
  </style></head><body>
    <div class="grid"></div>
    <div class="chip">🇮🇹 Nexi Group · ECR17 · LAN</div>
    <h1><span class="mono">ecr17-protocol</span></h1>
    <p>The Italian ECR17 payment protocol in pure Rust — a tested async engine for Nexi Group POS terminals, plus a Tauri desktop control panel.</p>
    <div class="tags"><span class="tag">🦀 Pure Rust core</span><span class="tag">🖥️ Tauri app</span><span class="tag">🛡️ Money-safe</span><span class="tag">✅ Fully tested</span></div>
    <div class="coin">💳</div>
  </body></html>`);
  await page.waitForTimeout(250);
  await page.screenshot({ path: resolve(resources, "banner.png") });
  await page.close();
  console.log("✓ banner.png");
}

// --- 2) App screenshots (needs the dev server on :1420) ----------------------
async function appPage() {
  const page = await browser.newPage({ viewport: { width: 1180, height: 820 }, deviceScaleFactor: 2 });
  await page.addInitScript(MOCK);
  await page.goto("http://localhost:1420", { waitUntil: "networkidle" });
  await page.evaluate(() => localStorage.clear());
  return page;
}

{
  // Main panel with a live log: connect, run status + a masked payment, emit progress/receipt.
  const page = await appPage();
  await page.getByTestId("cfg-host").fill("192.168.1.50");
  await page.evaluate(() => window.__ecr17mock.emit("ecr17:connection", "connected"));
  await page.evaluate(() =>
    window.__ecr17mock.setResponse("status", {
      terminalId: "12345678", terminalDateTime: "2026-07-12T09:41:00", status: 2, softwareRelease: "1.4.2",
    }),
  );
  await page.getByTestId("cmd-status").click();
  await page.evaluate(() =>
    window.__ecr17mock.setResponse("pay", {
      outcome: "ok", authCode: "004521", pan: "492917******0114", amountCents: 650,
      resultCode: "00", errorDescription: "",
    }),
  );
  await page.getByTestId("cmd-pay").click();
  await page.getByTestId("input-amountCents").fill("6.50");
  await page.getByTestId("sheet-submit").click();
  await page.evaluate(() => window.__ecr17mock.emit("ecr17:progress", { message: "INSERIRE CARTA" }));
  await page.evaluate(() => window.__ecr17mock.emit("ecr17:receipt", { text: "PAGAMENTO ESEGUITO" }));
  await page.waitForTimeout(400);
  await page.screenshot({ path: resolve(shots, "control-panel.png") });
  await page.close();
  console.log("✓ control-panel.png");
}

{
  // Command palette + the dynamic params sheet open on a financial command.
  const page = await appPage();
  await page.getByTestId("cfg-host").fill("192.168.1.50");
  await page.evaluate(() => window.__ecr17mock.emit("ecr17:connection", "connected"));
  await page.getByTestId("cmd-payExtended").click();
  await page.getByTestId("input-amountCents").fill("24.00");
  await page.waitForTimeout(300);
  await page.screenshot({ path: resolve(shots, "params-sheet.png") });
  await page.close();
  console.log("✓ params-sheet.png");
}

await browser.close();
console.log("done");
