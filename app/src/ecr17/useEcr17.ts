// React hook that owns the client lifecycle over the Tauri IPC and exposes a uniform `run`
// for the control panel. Wires native events to the logger and never throws to the UI.

import { useCallback, useEffect, useRef, useState } from "react";
import { log } from "./logger";
import { isFailure, safeDetail } from "./results";
import { ecr17, onConnection, onProgress, onReceipt } from "./tauri";
import type { ConnectionState, Ecr17Config, PaymentCardType } from "./types";

type Params = Record<string, unknown>;

export type RunStatus = "ok" | "ko" | "error";
export interface RunResult {
  status: RunStatus;
  result?: unknown;
}

export interface UseEcr17 {
  connectionState: ConnectionState;
  busy: boolean;
  lastProgress: string;
  connect: () => Promise<void>;
  disconnect: () => Promise<void>;
  run: (key: string, params: Params) => Promise<RunResult>;
}

function dispatch(key: string, p: Params): Promise<unknown> {
  const str = (k: string): string | undefined => {
    const v = p[k];
    return typeof v === "string" && v.length > 0 ? v : undefined;
  };
  const num = (k: string): number => (typeof p[k] === "number" ? (p[k] as number) : 0);
  const bool = (k: string): boolean => p[k] === true;
  const card = (): PaymentCardType | undefined =>
    typeof p.paymentType === "string" ? (p.paymentType as PaymentCardType) : undefined;

  switch (key) {
    case "status":
      return ecr17.status();
    case "pay":
      return ecr17.pay({
        amountCents: num("amountCents"),
        paymentType: card(),
        cardAlreadyPresent: bool("cardAlreadyPresent"),
        receiptText: str("receiptText"),
      });
    case "payExtended":
      return ecr17.payExtended({
        amountCents: num("amountCents"),
        paymentType: card(),
        cardAlreadyPresent: bool("cardAlreadyPresent"),
        receiptText: str("receiptText"),
      });
    case "reverse":
      return ecr17.reverse({ stan: str("stan") });
    case "preAuth":
      return ecr17.preAuth({
        amountCents: num("amountCents"),
        paymentType: card(),
        cardAlreadyPresent: bool("cardAlreadyPresent"),
        receiptText: str("receiptText"),
      });
    case "incrementalAuth":
      return ecr17.incrementalAuth({
        amountCents: num("amountCents"),
        originalPreAuthCode: str("originalPreAuthCode") ?? "",
        receiptText: str("receiptText"),
      });
    case "preAuthClosure":
      return ecr17.preAuthClosure({
        amountCents: num("amountCents"),
        originalPreAuthCode: str("originalPreAuthCode") ?? "",
        receiptText: str("receiptText"),
      });
    case "verifyCard":
      return ecr17.verifyCard({ paymentType: card() });
    case "closeSession":
      return ecr17.closeSession();
    case "totals":
      return ecr17.totals();
    case "sendLastResult":
      return ecr17.sendLastResult();
    case "enableEcrPrinting":
      return ecr17.enableEcrPrinting(bool("enabled"));
    case "reprint":
      return ecr17.reprint(bool("toEcr"));
    case "vas":
      return ecr17.vas(str("xmlRequest") ?? "");
    default:
      return Promise.reject(new Error(`Unknown command: ${key}`));
  }
}

export function useEcr17(config: Ecr17Config): UseEcr17 {
  const [connectionState, setConnectionState] = useState<ConnectionState>("disconnected");
  const [busy, setBusy] = useState(false);
  const [lastProgress, setLastProgress] = useState("");

  const configRef = useRef(config);
  configRef.current = config;
  // Serialized form of the config last pushed to the backend, so we re-`configure` only when
  // the operator actually changed something (configure resets the transport).
  const appliedRef = useRef("");

  // Subscribe to backend events once.
  useEffect(() => {
    let alive = true;
    const unlisteners: Array<() => void> = [];
    const track = (p: Promise<() => void>) => {
      p.then((u) => {
        if (alive) {
          unlisteners.push(u);
        } else {
          u();
        }
      }).catch(() => {});
    };
    track(
      onConnection((s) => {
        setConnectionState(s);
        log("info", `connection: ${s}`);
      }),
    );
    track(
      onProgress((e) => {
        setLastProgress(e.message);
        log("progress", e.message);
      }),
    );
    track(onReceipt((l) => log("receipt", l.text)));
    return () => {
      alive = false;
      for (const u of unlisteners) {
        u();
      }
    };
  }, []);

  // Push the latest form config into the backend before connecting/running.
  const applyConfig = useCallback(async () => {
    const serialized = JSON.stringify(configRef.current);
    if (serialized !== appliedRef.current) {
      await ecr17.configure(configRef.current);
      appliedRef.current = serialized;
    }
  }, []);

  const connect = useCallback(async () => {
    if (!configRef.current.host.trim()) {
      log("error", "connect failed", "Host is empty — enter the POS IP address first");
      return;
    }
    setBusy(true);
    log("sent", "connect()", JSON.stringify(configRef.current));
    try {
      await applyConfig();
      await ecr17.connect();
      log("ok", "connected");
    } catch (e) {
      log("error", "connect failed", String(e));
    } finally {
      setBusy(false);
    }
  }, [applyConfig]);

  const disconnect = useCallback(async () => {
    try {
      await ecr17.disconnect();
      log("info", "disconnect()");
    } catch (e) {
      log("error", "disconnect failed", String(e));
    }
  }, []);

  const run = useCallback(
    async (key: string, params: Params): Promise<RunResult> => {
      setBusy(true);
      setLastProgress("");
      log("sent", key, JSON.stringify(params));
      try {
        await applyConfig();
        const result = await dispatch(key, params);
        const failed = isFailure(result); // handles outcome + VAS responseId
        log(failed ? "ko" : "ok", `${key} →`, safeDetail(result)); // PAN masked
        return { status: failed ? "ko" : "ok", result };
      } catch (e) {
        log("error", `${key} failed`, String(e));
        return { status: "error" };
      } finally {
        setBusy(false);
      }
    },
    [applyConfig],
  );

  return { connectionState, busy, lastProgress, connect, disconnect, run };
}
