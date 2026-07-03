//! Stage 2 / A.1 — Stream lease + pull-result types.
//!
//! These are the high-level types that flow across the stream API once the
//! pull-based lease protocol is wired up. They live alongside the lower-level
//! IPC types in [`crate::ipc_types`], but are kept in a dedicated module so
//! the lease semantics are easy to reason about.
//!
//! ## Protocol summary
//!
//! ```text
//! Host                  Runner (extension process)
//! ----                  -------------------------
//! StreamOpen ---------->                            (capability, params, ...)
//!             <------- StreamOpened (lease_id, ...)
//! StreamPull --------->                            (lease_id, timeout_ms)
//!             <------- Success { StreamPullResult::Chunk }
//! StreamPull --------->                            (lease_id, timeout_ms)
//!             <------- Success { StreamPullResult::End { reason, .. } }
//! StreamClose -------->                            (lease_id)
//!             <------- Success { closed: true }
//! ```
//!
//! All pull / cancel / close responses are wrapped in
//! [`crate::ipc_types::IpcResponse::Success`] so the host's existing
//! `request_id`-based response router can correlate them.

use serde::{Deserialize, Serialize};

use crate::ipc_types::{StreamChunkPayload, StreamEndReason, StreamTransportInfo};

/// Result of a single `StreamPull` call.
///
/// Serialized as the `data` field of `IpcResponse::Success`. Tagged with
/// `kind` so the host can pattern-match on a single discriminated union
/// rather than parsing optional fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamPullResult {
    /// A chunk was available within the timeout window.
    Chunk {
        /// The chunk payload (binary / text / json / end-of-stream sentinel).
        chunk: StreamChunkPayload,
    },
    /// The stream ended naturally (source signaled completion) or with error.
    /// After this, the lease is gone — subsequent pulls return [`LeaseGone`].
    End {
        /// Why the stream ended.
        reason: StreamEndReason,
        /// Optional error message (set when `reason == Error`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// No chunk arrived within `timeout_ms`. The lease is still open; the
    /// host may pull again. Useful for backpressure-aware consumers that
    /// want to interleave other work between pulls.
    Timeout,
    /// The lease no longer exists (closed, cancelled, or expired). Subsequent
    /// pulls will keep returning this — the host should stop pulling.
    LeaseGone,
}

/// Information returned by a successful `StreamOpen` — the host-side mirror
/// of [`crate::ipc_types::IpcResponse::StreamOpened`], parsed into a typed
/// struct for ergonomic consumption by host callers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamOpenedInfo {
    /// Newly minted lease identifier.
    pub lease_id: String,
    /// Capability-specific initial metadata (e.g. `{ "session_id": 42 }`).
    pub initial_metadata: serde_json::Value,
    /// Negotiated data-plane transport info (host may downgrade from the
    /// extension's requested [`crate::ipc_types::StreamTransport`]).
    pub transport: StreamTransportInfo,
}
