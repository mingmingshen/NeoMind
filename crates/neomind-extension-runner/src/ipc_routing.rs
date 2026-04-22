//! IPC routing for capability invocation and message dispatch
//!
//! A background thread reads all stdin messages and routes them to:
//! 1. Pending capability requests (via PENDING_REQUESTS)
//! 2. Main event queue (via EVENT_TX)

use std::collections::HashMap;
use std::io::Read;
use std::panic::UnwindSafe;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use serde_json::json;
use tracing::{debug, error, trace, warn};

use neomind_extension_sdk::{IpcMessage, IpcResponse};

type ResponseSender = Sender<IpcResponse>;

/// Pending capability requests: request_id -> response sender
static PENDING_REQUESTS: std::sync::OnceLock<Mutex<HashMap<u64, ResponseSender>>> =
    std::sync::OnceLock::new();

/// Channel-based event queue for main loop (replaces polling)
static EVENT_TX: std::sync::OnceLock<tokio::sync::mpsc::UnboundedSender<IpcMessage>> =
    std::sync::OnceLock::new();

/// Global mutex for stdout writes — prevents interleaved frames from
/// `send_response()` and the push-output callback running concurrently.
pub(crate) static STDOUT_WRITE_MUTEX: Mutex<()> = Mutex::new(());

/// Wrap an FFI call in `catch_unwind` so an extension panic does not
/// abort the runner process. Returns the value or an error string.
pub(crate) fn safe_ffi_call<F, T>(label: &str, f: F) -> Result<T, String>
where
    F: FnOnce() -> T + UnwindSafe,
{
    std::panic::catch_unwind(f).map_err(|payload| {
        let msg = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };
        error!(label = label, panic = %msg, "Extension FFI call panicked");
        format!("Extension panicked in {}: {}", label, msg)
    })
}

pub(crate) fn get_pending_requests() -> &'static Mutex<HashMap<u64, ResponseSender>> {
    PENDING_REQUESTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Create the event channel and return the receiver (call once at startup)
pub(crate) fn create_event_channel() -> tokio::sync::mpsc::UnboundedReceiver<IpcMessage> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    EVENT_TX.set(tx).expect("event channel already initialized");
    rx
}

/// Register a pending request and return the response receiver
pub(crate) fn register_pending_request(request_id: u64) -> Receiver<IpcResponse> {
    let (tx, rx) = channel();
    get_pending_requests()
        .lock()
        .unwrap()
        .insert(request_id, tx);
    rx
}

/// Complete a pending request with the response
pub(crate) fn complete_pending_request(request_id: u64, response: IpcResponse) {
    if let Some(tx) = get_pending_requests().lock().unwrap().remove(&request_id) {
        let _ = tx.send(response);
    }
}

/// Push an event to the channel for main loop processing
pub(crate) fn push_event(message: IpcMessage) {
    if let Some(tx) = EVENT_TX.get() {
        let _ = tx.send(message);
    }
}

/// Start the stdin reader thread
/// This thread reads all messages from stdin and routes them appropriately
pub(crate) fn start_stdin_reader() -> std::thread::JoinHandle<()> {
    std::thread::spawn(|| {
        debug!("StdinReader started");

        let mut consecutive_errors = 0u32;
        const MAX_CONSECUTIVE_ERRORS: u32 = 5;

        loop {
            // Read length prefix
            let mut len_bytes = [0u8; 4];
            match std::io::stdin().read_exact(&mut len_bytes) {
                Ok(_) => {
                    consecutive_errors = 0;
                }
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    debug!("Stdin closed");
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                    // Retry on interrupt (signal received during read)
                    continue;
                }
                Err(e) => {
                    consecutive_errors += 1;
                    warn!(
                        consecutive_errors,
                        max = MAX_CONSECUTIVE_ERRORS,
                        "Error reading length: {e}"
                    );
                    if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                        error!("Too many consecutive stdin errors, giving up");
                        break;
                    }
                    // Brief backoff before retry
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    continue;
                }
            }

            let len = u32::from_le_bytes(len_bytes) as usize;
            let max_size = std::env::var("NEOMIND_IPC_MAX_SIZE")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(10 * 1024 * 1024);
            if len > max_size {
                warn!(len, max_size, "Message too large, draining");
                // Drain the oversized payload in chunks to keep stdin aligned
                let mut remaining = len;
                let mut drain_buf = [0u8; 4096];
                while remaining > 0 {
                    let to_read = remaining.min(drain_buf.len());
                    if std::io::stdin().read_exact(&mut drain_buf[..to_read]).is_err() {
                        debug!("Stdin closed while draining oversized message");
                        break;
                    }
                    remaining -= to_read;
                }
                continue;
            }

            // Read payload
            let mut payload = vec![0u8; len];
            if let Err(e) = std::io::stdin().read_exact(&mut payload) {
                warn!("Error reading payload: {e}");
                continue;
            }

            // Parse message
            let message: IpcMessage = match IpcMessage::from_bytes(&payload) {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to parse IPC message: {e}");
                    continue;
                }
            };

            // Route message
            match &message {
                IpcMessage::CapabilityResult { request_id, .. } => {
                    // Convert to IpcResponse and route to waiting invoke()
                    let response = IpcResponse::CapabilityResult {
                        request_id: *request_id,
                        result: match message {
                            IpcMessage::CapabilityResult { ref result, .. } => result.clone(),
                            _ => json!({}),
                        },
                        error: match message {
                            IpcMessage::CapabilityResult { ref error, .. } => error.clone(),
                            _ => None,
                        },
                    };
                    complete_pending_request(*request_id, response);
                    trace!(request_id, "Routed CapabilityResult");
                }
                _ => {
                    // Push to event queue for main loop
                    push_event(message);
                }
            }
        }

        debug!("StdinReader exiting");
    })
}
