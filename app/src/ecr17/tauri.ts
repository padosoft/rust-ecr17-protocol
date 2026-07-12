// Typed wrappers around the Tauri IPC. Command names are the backend's snake_case function
// names; argument objects use camelCase keys (Tauri v2 maps them to the Rust snake_case
// parameters).

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  CardVerificationRequest,
  CardVerificationResult,
  CloseSessionResult,
  ConnectionState,
  Ecr17Config,
  IncrementalAuthRequest,
  PaymentRequest,
  PaymentResult,
  PosStatusResponse,
  PreAuthClosureRequest,
  PreAuthRequest,
  PreAuthResult,
  ProgressEvent,
  ReceiptLine,
  ReversalRequest,
  ReversalResult,
  TotalsResult,
  VasResult,
} from "./types";

export const EVENT_PROGRESS = "ecr17:progress";
export const EVENT_RECEIPT = "ecr17:receipt";
export const EVENT_CONNECTION = "ecr17:connection";

export const ecr17 = {
  configure: (config: Ecr17Config) => invoke<void>("configure", { config }),
  configuration: () => invoke<Ecr17Config | null>("configuration"),
  connect: () => invoke<void>("connect"),
  disconnect: () => invoke<void>("disconnect"),
  isConnected: () => invoke<boolean>("is_connected"),

  status: () => invoke<PosStatusResponse>("status"),
  pay: (request: PaymentRequest) => invoke<PaymentResult>("pay", { request }),
  payExtended: (request: PaymentRequest) => invoke<PaymentResult>("pay_extended", { request }),
  reverse: (request: ReversalRequest) => invoke<ReversalResult>("reverse", { request }),
  preAuth: (request: PreAuthRequest) => invoke<PreAuthResult>("pre_auth", { request }),
  incrementalAuth: (request: IncrementalAuthRequest) =>
    invoke<PreAuthResult>("incremental_auth", { request }),
  preAuthClosure: (request: PreAuthClosureRequest) =>
    invoke<PaymentResult>("pre_auth_closure", { request }),
  verifyCard: (request: CardVerificationRequest) =>
    invoke<CardVerificationResult>("verify_card", { request }),
  closeSession: () => invoke<CloseSessionResult>("close_session"),
  totals: () => invoke<TotalsResult>("totals"),
  sendLastResult: () => invoke<PaymentResult>("send_last_result"),
  enableEcrPrinting: (enabled: boolean) => invoke<void>("enable_ecr_printing", { enabled }),
  reprint: (toEcr: boolean) => invoke<void>("reprint", { toEcr }),
  vas: (xmlRequest: string) => invoke<VasResult>("vas", { xmlRequest }),
};

export function onProgress(cb: (e: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>(EVENT_PROGRESS, (e) => cb(e.payload));
}

export function onReceipt(cb: (l: ReceiptLine) => void): Promise<UnlistenFn> {
  return listen<ReceiptLine>(EVENT_RECEIPT, (e) => cb(e.payload));
}

export function onConnection(cb: (s: ConnectionState) => void): Promise<UnlistenFn> {
  return listen<ConnectionState>(EVENT_CONNECTION, (e) => cb(e.payload));
}
