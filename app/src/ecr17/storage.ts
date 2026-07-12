// Persists the last-used Ecr17Config to localStorage so the panel isn't re-filled each launch.

import type { Ecr17Config } from "./types";

const CONFIG_KEY = "ecr17.config";

export const DEFAULT_CONFIG: Ecr17Config = {
  host: "",
  port: 10000,
  terminalId: "",
  cashRegisterId: "",
  lrcMode: "std",
  keepAlive: true,
  autoReconnect: true,
  connectionTimeoutMs: 5000,
  responseTimeoutMs: 60000,
  ackTimeoutMs: 2000,
  retryCount: 3,
  retryDelayMs: 200,
  receiptDrainMs: 0,
  debug: true,
};

export function loadConfig(): Ecr17Config {
  try {
    const raw = localStorage.getItem(CONFIG_KEY);
    if (!raw) {
      return { ...DEFAULT_CONFIG };
    }
    const parsed = JSON.parse(raw) as Partial<Ecr17Config>;
    return { ...DEFAULT_CONFIG, ...parsed };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

export function saveConfig(config: Ecr17Config): void {
  try {
    localStorage.setItem(CONFIG_KEY, JSON.stringify(config));
  } catch {
    // best-effort persistence
  }
}
