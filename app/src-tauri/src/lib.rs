//! ECR17 Control Panel — Tauri backend.
//!
//! Holds one [`Ecr17Client`] over the real tokio TCP transport in managed state and exposes
//! one `#[tauri::command]` per protocol command to the React UI. The client's
//! progress/receipt/connection-state callbacks are forwarded to the webview as Tauri events
//! (`ecr17:progress`, `ecr17:receipt`, `ecr17:connection`). Errors are surfaced to the UI as
//! strings; the frontend handles PAN masking for display.

use std::time::Duration;

use ecr17_protocol::error::Ecr17Error;
use ecr17_protocol::transport::tcp::TcpTransport;
use ecr17_protocol::types::{
    CardVerificationRequest, CardVerificationResult, CloseSessionResult, Ecr17Config,
    IncrementalAuthRequest, PaymentRequest, PaymentResult, PosStatusResponse,
    PreAuthClosureRequest, PreAuthRequest, PreAuthResult, ReversalRequest, ReversalResult,
    TotalsResult, VasResult,
};
use ecr17_protocol::Ecr17Client;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

type Client = Ecr17Client<TcpTransport>;

/// Event channels emitted to the webview.
const EVENT_PROGRESS: &str = "ecr17:progress";
const EVENT_RECEIPT: &str = "ecr17:receipt";
const EVENT_CONNECTION: &str = "ecr17:connection";

const NOT_CONFIGURED: &str = "ECR17: not configured — call configure() first";

/// Managed application state: the (optional, until configured) client.
#[derive(Default)]
struct AppState {
    client: Mutex<Option<Client>>,
}

fn err(e: Ecr17Error) -> String {
    e.to_string()
}

/// Emits a UI event, logging (not swallowing) a failure. An emit failure is only expected
/// when the webview is gone (app closing), which is non-recoverable here — but it is logged
/// to stderr rather than silently dropped.
fn emit<T: serde::Serialize + Clone>(app: &AppHandle, event: &str, payload: T) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("ecr17: failed to emit '{event}': {e}");
    }
}

/// Builds a client from `config` and wires its callbacks to Tauri events.
fn build_client(app: &AppHandle, config: Ecr17Config) -> Client {
    let host = config.host.clone();
    let port = config.port.unwrap_or(10_000);
    let timeout = Duration::from_millis(u64::from(config.connection_timeout_ms.unwrap_or(5000)));
    let client = Ecr17Client::new(TcpTransport::new(host, port, timeout), config);

    let h = app.clone();
    client.set_on_progress(move |e| emit(&h, EVENT_PROGRESS, e));
    let h = app.clone();
    client.set_on_receipt_line(move |l| emit(&h, EVENT_RECEIPT, l));
    let h = app.clone();
    client.set_on_connection_state_change(move |s| emit(&h, EVENT_CONNECTION, s));
    client
}

// --- Configuration & connection ---

#[tauri::command]
async fn configure(
    app: AppHandle,
    state: State<'_, AppState>,
    config: Ecr17Config,
) -> Result<(), String> {
    let client = build_client(&app, config);
    let mut guard = state.client.lock().await;
    // Cleanly close any previous connection before replacing the client, so reconfiguring
    // doesn't drop a live TCP stream abruptly.
    if let Some(old) = guard.as_mut() {
        old.disconnect().await;
    }
    *guard = Some(client);
    Ok(())
}

#[tauri::command]
async fn configuration(state: State<'_, AppState>) -> Result<Option<Ecr17Config>, String> {
    Ok(state
        .client
        .lock()
        .await
        .as_ref()
        .map(|c| c.configuration().clone()))
}

#[tauri::command]
async fn connect(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.client.lock().await;
    guard
        .as_mut()
        .ok_or_else(|| NOT_CONFIGURED.to_string())?
        .connect()
        .await
        .map_err(err)
}

#[tauri::command]
async fn disconnect(state: State<'_, AppState>) -> Result<(), String> {
    if let Some(client) = state.client.lock().await.as_mut() {
        client.disconnect().await;
    }
    Ok(())
}

#[tauri::command]
async fn is_connected(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state
        .client
        .lock()
        .await
        .as_ref()
        .is_some_and(|c| c.is_connected()))
}

// --- Commands (one per protocol command) ---

macro_rules! command {
    // No-argument command.
    ($name:ident, $ret:ty, |$c:ident| $call:expr) => {
        #[tauri::command]
        async fn $name(state: State<'_, AppState>) -> Result<$ret, String> {
            let mut guard = state.client.lock().await;
            let $c = guard.as_mut().ok_or_else(|| NOT_CONFIGURED.to_string())?;
            $call.await.map_err(err)
        }
    };
    // Command taking a typed `request`.
    ($name:ident, $req:ty, $ret:ty, |$c:ident, $r:ident| $call:expr) => {
        #[tauri::command]
        async fn $name(state: State<'_, AppState>, request: $req) -> Result<$ret, String> {
            let mut guard = state.client.lock().await;
            let $c = guard.as_mut().ok_or_else(|| NOT_CONFIGURED.to_string())?;
            let $r = &request;
            $call.await.map_err(err)
        }
    };
}

command!(status, PosStatusResponse, |c| c.status());
command!(pay, PaymentRequest, PaymentResult, |c, r| c.pay(r));
command!(pay_extended, PaymentRequest, PaymentResult, |c, r| c
    .pay_extended(r));
command!(reverse, ReversalRequest, ReversalResult, |c, r| c
    .reverse(r));
command!(pre_auth, PreAuthRequest, PreAuthResult, |c, r| c
    .pre_auth(r));
command!(
    incremental_auth,
    IncrementalAuthRequest,
    PreAuthResult,
    |c, r| c.incremental_auth(r)
);
command!(
    pre_auth_closure,
    PreAuthClosureRequest,
    PaymentResult,
    |c, r| c.pre_auth_closure(r)
);
command!(
    verify_card,
    CardVerificationRequest,
    CardVerificationResult,
    |c, r| c.verify_card(r)
);
command!(close_session, CloseSessionResult, |c| c.close_session());
command!(totals, TotalsResult, |c| c.totals());
command!(send_last_result, PaymentResult, |c| c.send_last_result());

#[tauri::command]
async fn enable_ecr_printing(state: State<'_, AppState>, enabled: bool) -> Result<(), String> {
    let mut guard = state.client.lock().await;
    let c = guard.as_mut().ok_or_else(|| NOT_CONFIGURED.to_string())?;
    c.enable_ecr_printing(enabled).await.map_err(err)
}

#[tauri::command]
async fn reprint(state: State<'_, AppState>, to_ecr: bool) -> Result<(), String> {
    let mut guard = state.client.lock().await;
    let c = guard.as_mut().ok_or_else(|| NOT_CONFIGURED.to_string())?;
    c.reprint(to_ecr).await.map_err(err)
}

#[tauri::command]
async fn vas(state: State<'_, AppState>, xml_request: String) -> Result<VasResult, String> {
    let mut guard = state.client.lock().await;
    let c = guard.as_mut().ok_or_else(|| NOT_CONFIGURED.to_string())?;
    c.vas(&xml_request).await.map_err(err)
}

/// Scaffold placeholder invoked by the default `App.tsx`; removed in MACRO 7 with the
/// scaffold UI. Kept for now so the scaffold's Greet button doesn't error before the real
/// control panel lands.
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            configure,
            configuration,
            connect,
            disconnect,
            is_connected,
            status,
            pay,
            pay_extended,
            reverse,
            pre_auth,
            incremental_auth,
            pre_auth_closure,
            verify_card,
            close_session,
            totals,
            send_last_result,
            enable_ecr_printing,
            reprint,
            vas,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_maps_to_nonempty_string() {
        // Assert the shape (non-empty, mentions the failure), not the exact Display copy,
        // so harmless message wording changes don't break the backend test.
        let s = err(Ecr17Error::Disconnected);
        assert!(!s.is_empty());
        assert!(s.to_lowercase().contains("disconnect"), "{s}");
    }

    #[test]
    fn config_round_trips_camel_case_over_ipc() {
        // The IPC payload from the UI is camelCase JSON; it must deserialize into Ecr17Config.
        let json =
            r#"{"host":"10.0.0.5","port":10000,"terminalId":"12345678","cashRegisterId":"1"}"#;
        let cfg: Ecr17Config = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.host, "10.0.0.5");
        assert_eq!(cfg.terminal_id, "12345678");
        assert_eq!(cfg.port, Some(10000));
    }

    #[test]
    fn config_serializes_camel_case_for_the_ui() {
        // configuration() serializes the stored config back to the UI — keys must be camelCase.
        let cfg: Ecr17Config =
            serde_json::from_str(r#"{"host":"h","terminalId":"t","cashRegisterId":"c"}"#).unwrap();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains(r#""terminalId":"t""#), "{json}");
        assert!(json.contains(r#""cashRegisterId":"c""#), "{json}");
    }

    #[test]
    fn payment_request_deserializes_from_ui_payload() {
        let json = r#"{"amountCents":650,"paymentType":"credit"}"#;
        let req: PaymentRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.amount_cents, 650);
    }
}
