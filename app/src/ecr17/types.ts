// TypeScript mirror of the ecr17-protocol serde data model (camelCase over the Tauri IPC).

export type LrcMode = "stx" | "std" | "noext" | "stx_noext";
export type ConnectionState = "disconnected" | "connecting" | "connected";
export type TransactionOutcome = "ok" | "ko" | "cardNotPresent" | "unknownTag" | "unknown";
export type CardType = "debit" | "credit" | "other" | "unknown";
export type TransactionEntryMode = "icc" | "mag" | "manual" | "clessMag" | "clessIcc" | "unknown";
export type PaymentCardType = "auto" | "debit" | "credit" | "other";
export type TokenizationService = "recurring" | "unscheduledOrOneClick";

export interface TokenizationRequest {
  service: TokenizationService;
  contractCode: string;
}

export interface Ecr17Config {
  host: string;
  port?: number;
  terminalId: string;
  cashRegisterId: string;
  lrcMode?: LrcMode;
  keepAlive?: boolean;
  autoReconnect?: boolean;
  connectionTimeoutMs?: number;
  responseTimeoutMs?: number;
  ackTimeoutMs?: number;
  receiptDrainMs?: number;
  retryCount?: number;
  retryDelayMs?: number;
  debug?: boolean;
}

export interface PosStatusResponse {
  terminalId: string;
  terminalDateTime: string; // ISO 8601
  status: number;
  softwareRelease: string;
}

export interface PaymentRequest {
  amountCents: number;
  cashRegisterId?: string;
  paymentType?: PaymentCardType;
  cardAlreadyPresent?: boolean;
  receiptText?: string;
  tokenization?: TokenizationRequest;
}

export interface ReversalRequest {
  cashRegisterId?: string;
  stan?: string;
}

export interface PreAuthRequest {
  amountCents: number;
  cashRegisterId?: string;
  paymentType?: PaymentCardType;
  cardAlreadyPresent?: boolean;
  receiptText?: string;
  tokenization?: TokenizationRequest;
}

export interface IncrementalAuthRequest {
  amountCents: number;
  originalPreAuthCode: string;
  cashRegisterId?: string;
  receiptText?: string;
}

export interface PreAuthClosureRequest {
  amountCents: number;
  originalPreAuthCode: string;
  cashRegisterId?: string;
  receiptText?: string;
}

export interface CardVerificationRequest {
  cashRegisterId?: string;
  paymentType?: PaymentCardType;
  tokenization?: TokenizationRequest;
}

export interface CurrencyExchange {
  applied: boolean;
  rate?: number;
  currencyCode?: string;
  amountCents?: number;
  precision?: number;
}

export interface PaymentResult {
  outcome: TransactionOutcome;
  resultCode: string;
  pan?: string;
  entryMode?: TransactionEntryMode;
  authCode?: string;
  hostDateTime?: string;
  cardType?: CardType;
  acquirerId?: string;
  stan?: string;
  onlineId?: string;
  errorDescription?: string;
  currencyExchange?: CurrencyExchange;
}

export interface ReversalResult {
  outcome: TransactionOutcome;
  resultCode: string;
  pan?: string;
  entryMode?: TransactionEntryMode;
  hostDateTime?: string;
  cardType?: CardType;
  acquirerId?: string;
  stan?: string;
  onlineId?: string;
  actionCode?: string;
  errorDescription?: string;
}

export interface PreAuthResult {
  outcome: TransactionOutcome;
  resultCode: string;
  pan?: string;
  entryMode?: TransactionEntryMode;
  authCode?: string;
  preAuthorizedAmountCents?: number;
  preAuthCode?: string;
  actionCode?: string;
  hostDateTime?: string;
  cardType?: CardType;
  acquirerId?: string;
  stan?: string;
  onlineId?: string;
  errorDescription?: string;
}

export interface CardVerificationResult {
  outcome: TransactionOutcome;
  resultCode: string;
  pan?: string;
  entryMode?: TransactionEntryMode;
  authCode?: string;
  hostDateTime?: string;
  cardType?: CardType;
  acquirerId?: string;
  stan?: string;
  onlineId?: string;
  actionCode?: string;
  errorDescription?: string;
}

export interface TotalsResult {
  outcome: TransactionOutcome;
  resultCode: string;
  posTotalCents: number;
}

export interface CloseSessionResult {
  outcome: TransactionOutcome;
  resultCode: string;
  posTotalCents?: number;
  hostTotalCents?: number;
  actionCode?: string;
  errorDescription?: string;
}

export interface VasResult {
  responseId: string;
  responseMessage: string;
  orderId?: string;
  rawXml: string;
}

export interface ProgressEvent {
  message: string;
}

export interface ReceiptLine {
  text: string;
}
