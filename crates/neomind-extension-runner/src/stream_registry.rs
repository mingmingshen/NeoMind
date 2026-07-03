//! Stage 2 / A.2 — Stream lease registry for the pull-based lease protocol.
//!
//! The registry owns one [`StreamLease`] per active stream. Leases are keyed
//! by a host-generated `lease_id`. The host capability (e.g. ChatStream)
//! produces chunks that arrive at the runner as `AgentStreamChunk` events
//! (via `IpcMessage::EventPush`); the pump task spawned at open time routes
//! those chunks into the lease's mpsc channel. When the host calls
//! `StreamPull`, we pull from that channel with the requested timeout and
//! return a [`StreamPullResult`].
//!
//! ## Lifecycle
//!
//! - [`open`](StreamLeaseRegistry::open) — mint lease_id, create channels,
//!   store lease. The caller (runner main) is responsible for the actual
//!   capability invocation that starts chunk production.
//! - [`route_chunk`](StreamLeaseRegistry::route_chunk) — pump task calls this
//!   for each `AgentStreamChunk` event whose session_id matches the lease.
//! - [`pull`](StreamLeaseRegistry::pull) — host calls StreamPull; returns a
//!   [`StreamPullResult`] (Chunk / End / Timeout / LeaseGone).
//! - [`signal_end`](StreamLeaseRegistry::signal_end) — pump task or cancel
//!   handler marks the lease as terminal so the next pull returns End.
//! - [`cancel`](StreamLeaseRegistry::cancel) — flips the cancel watch;
//!   capability provider observes it and aborts.
//! - [`close`](StreamLeaseRegistry::close) — removes the lease; subsequent
//!   pulls return LeaseGone.

use std::collections::HashMap;
use std::sync::Arc;

use neomind_extension_sdk::ipc::{StreamChunkPayload, StreamEndReason};
use neomind_extension_sdk::StreamPullResult;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, warn};

#[cfg(all(unix, feature = "shm-ring"))]
use neomind_extension_sdk::shm_ring::{DropPolicy as RingDropPolicy, RingHandle, WriteResult};

/// Capacity for the per-lease chunk channel. Bounded to apply backpressure
/// on the producing capability (host) when the extension consumer is slow.
/// The lease's `buffer_size` from `StreamOpen` overrides this.
const DEFAULT_CHUNK_CHANNEL_CAPACITY: usize = 8;

/// Internal max for the channel capacity — defends against an extension
/// asking for an absurdly large buffer.
const MAX_CHUNK_CHANNEL_CAPACITY: usize = 256;

/// A single open stream lease.
pub struct StreamLease {
    /// Unique identifier returned to the host in `StreamOpened`.
    pub lease_id: String,
    /// Host capability session_id, used to filter `AgentStreamChunk` events.
    /// Set by the runner once the capability invocation returns.
    pub session_id: Option<String>,
    /// Sender held by the registry's pump task to push chunks into the queue.
    /// Wrapped in Option so we can close it from the producer side on End.
    pub chunk_tx: Option<mpsc::Sender<StreamChunkPayload>>,
    /// Receiver drained by `pull()`. Owned by the lease so only one consumer
    /// exists per lease.
    pub chunk_rx: mpsc::Receiver<StreamChunkPayload>,
    /// Cancel watch — flipped to `true` by `cancel()`. The capability
    /// provider (or the runner's pump task) observes this and aborts.
    pub cancel_tx: watch::Sender<bool>,
    /// Receiver for the cancel signal, cloned for observers.
    pub cancel_rx: watch::Receiver<bool>,
    /// Set when the stream reached a terminal state (Completed / Error /
    /// Cancelled / HostShutdown / LeaseExpired). The next pull observes this
    /// and returns [`StreamPullResult::End`] (once) or [`LeaseGone`] after.
    pub end_reason: Option<StreamEndReason>,
    /// Optional error message paired with `end_reason == Error`.
    pub end_error: Option<String>,
    /// Whether the End has been delivered to a puller yet. The first pull
    /// after `end_reason` is set returns `End`; subsequent ones return
    /// `LeaseGone`. This implements the spec'd one-shot End delivery.
    pub end_delivered: bool,
    /// Optional handle to the pump task that routes `EventPush` → mpsc.
    /// Aborted on close.
    pub pump_handle: Option<JoinHandle<()>>,
    /// Optional SHM ring (Phase B / B.1 fast path). When present, chunks
    /// are written directly to the ring via PcmRingWriter instead of the
    /// mpsc channel. The host opens its own reader using the shm_name
    /// returned in `StreamOpened`. Pull-based consumption is still allowed
    /// but is redundant — the host should read from the ring directly for
    /// high-throughput streams.
    #[cfg(all(unix, feature = "shm-ring"))]
    pub ring: Option<Arc<RingHandle>>,
}

impl StreamLease {
    /// Create a fresh lease with a new chunk channel and cancel watch.
    pub fn new(lease_id: String, buffer_size: u32) -> Self {
        let cap = (buffer_size as usize)
            .clamp(1, MAX_CHUNK_CHANNEL_CAPACITY)
            .max(DEFAULT_CHUNK_CHANNEL_CAPACITY);
        let (chunk_tx, chunk_rx) = mpsc::channel(cap);
        let (cancel_tx, cancel_rx) = watch::channel(false);
        Self {
            lease_id,
            session_id: None,
            chunk_tx: Some(chunk_tx),
            chunk_rx,
            cancel_tx,
            cancel_rx,
            end_reason: None,
            end_error: None,
            end_delivered: false,
            pump_handle: None,
            #[cfg(all(unix, feature = "shm-ring"))]
            ring: None,
        }
    }

    /// Attach a SHM ring (B.1 fast path). Subsequent route_by_session calls
    /// will write directly to the ring instead of the mpsc channel.
    #[cfg(all(unix, feature = "shm-ring"))]
    pub fn set_ring(&mut self, ring: Arc<RingHandle>) {
        self.ring = Some(ring);
    }

    /// Whether cancel was triggered.
    pub fn is_cancelled(&self) -> bool {
        *self.cancel_tx.borrow()
    }
}

/// Concurrent map of active leases. Behind an Arc<Mutex> so the pump task
/// and pull handlers can access it from different tasks.
#[derive(Default)]
pub struct StreamLeaseRegistry {
    leases: Mutex<HashMap<String, Arc<Mutex<StreamLease>>>>,
    /// Reverse index: host capability `session_id` → `lease_id`. Populated
    /// when [`set_session_id`] is called. Lets `route_by_session` be O(1)
    /// instead of scanning every lease on each AgentStreamChunk event.
    by_session: Mutex<HashMap<String, String>>,
}

impl StreamLeaseRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a new lease and return the lease_id. The caller is responsible
    /// for the capability invocation that produces chunks and for spawning
    /// the pump task that calls [`route_chunk`].
    pub async fn open(&self, lease_id: String, buffer_size: u32) -> Arc<Mutex<StreamLease>> {
        let lease = Arc::new(Mutex::new(StreamLease::new(lease_id.clone(), buffer_size)));
        self.leases.lock().await.insert(lease_id, lease.clone());
        lease
    }

    /// Attach the pump task handle so close() can abort it.
    pub async fn attach_pump(&self, lease_id: &str, handle: JoinHandle<()>) {
        if let Some(lease) = self.leases.lock().await.get(lease_id) {
            lease.lock().await.pump_handle = Some(handle);
        }
    }

    /// Set the session_id on a lease (after capability invocation returns).
    /// Also populates the reverse index used by [`route_by_session`].
    pub async fn set_session_id(&self, lease_id: &str, session_id: impl Into<String>) {
        let session_id = session_id.into();
        if let Some(lease) = self.leases.lock().await.get(lease_id) {
            lease.lock().await.session_id = Some(session_id.clone());
        }
        self.by_session.lock().await.insert(session_id, lease_id.to_string());
    }

    /// Attach a SHM ring handle to a lease (B.1 fast path).
    #[cfg(all(unix, feature = "shm-ring"))]
    pub async fn attach_ring(&self, lease_id: &str, ring: Arc<RingHandle>) -> bool {
        if let Some(lease) = self.leases.lock().await.get(lease_id) {
            lease.lock().await.set_ring(ring);
            true
        } else {
            false
        }
    }

    /// Route an AgentStreamChunk payload (already extracted from the event
    /// envelope) to the lease bound to this session_id. Returns the lease_id
    /// list that matched (typically zero or one). Empty when no lease is
    /// bound yet — caller may log/trace.
    ///
    /// If the lease has a SHM ring attached (B.1), the chunk is written
    /// directly to the ring instead of going through the mpsc channel.
    /// Json chunks are serialized to bytes before writing; binary chunks
    /// are written as-is.
    pub async fn route_by_session(
        &self,
        session_id: &str,
        chunk: serde_json::Value,
    ) -> Vec<String> {
        let lease_id = match self.by_session.lock().await.get(session_id) {
            Some(id) => id.clone(),
            None => return Vec::new(),
        };

        // Fast path: SHM ring. Skip the mpsc queue entirely.
        #[cfg(all(unix, feature = "shm-ring"))]
        {
            // Clone the lease Arc out so we don't hold the outer leases guard
            // while awaiting the inner per-lease lock.
            let lease_arc_opt: Option<Arc<Mutex<StreamLease>>> = {
                let leases = self.leases.lock().await;
                leases.get(&lease_id).cloned()
            };
            if let Some(lease_arc) = lease_arc_opt {
                let ring_opt = lease_arc.lock().await.ring.clone();
                if let Some(ring) = ring_opt {
                    let bytes = match serde_json::to_vec(&chunk) {
                        Ok(b) => b,
                        Err(e) => {
                            warn!(%lease_id, %session_id, error = %e, "Failed to serialize chunk for SHM ring");
                            return vec![lease_id];
                        }
                    };
                    let ts_ns = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_nanos() as u64)
                        .unwrap_or(0);
                    let writer = ring.writer();
                    match writer.write(&bytes, ts_ns) {
                        WriteResult::Written(_) | WriteResult::Dropped => {
                            return vec![lease_id];
                        }
                        WriteResult::InvalidSize => {
                            warn!(
                                %lease_id,
                                %session_id,
                                chunk_len = bytes.len(),
                                "Chunk larger than SHM frame_size_max — dropping"
                            );
                            return vec![lease_id];
                        }
                    }
                }
            }
        }

        // Default path: mpsc queue for IpcJson transport.
        let payload = StreamChunkPayload::Json(chunk);
        if self.route_chunk(&lease_id, payload).await {
            vec![lease_id]
        } else {
            self.by_session.lock().await.remove(session_id);
            Vec::new()
        }
    }

    /// Push a chunk into the lease's queue. Called by the pump task when an
    /// `AgentStreamChunk` event arrives matching this lease's session_id.
    /// Returns false if the lease is gone or the channel is closed (the
    /// stream has been torn down).
    pub async fn route_chunk(&self, lease_id: &str, chunk: StreamChunkPayload) -> bool {
        let lease = {
            let leases = self.leases.lock().await;
            match leases.get(lease_id) {
                Some(l) => l.clone(),
                None => return false,
            }
        };
        let tx = {
            let guard = lease.lock().await;
            match &guard.chunk_tx {
                Some(tx) => tx.clone(),
                None => return false,
            }
        };
        // Fast path: try_send to avoid blocking the pump task on a full
        // queue. If the queue is full (Block policy), fall back to async
        // send so we apply natural backpressure to the producing capability
        // rather than dropping the chunk.
        match tx.try_send(chunk) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(chunk)) => tx.send(chunk).await.is_ok(),
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    }

    /// Mark a lease terminal. The next pull returns End (one-shot); later
    /// pulls return LeaseGone.
    pub async fn signal_end(
        &self,
        lease_id: &str,
        reason: StreamEndReason,
        error: Option<String>,
    ) {
        if let Some(lease) = self.leases.lock().await.get(lease_id) {
            let mut g = lease.lock().await;
            // First writer wins — don't overwrite a Cancelled with Completed.
            if g.end_reason.is_none() {
                g.end_reason = Some(reason);
                g.end_error = error;
            }
            // Close the producer side so route_chunk stops accepting.
            g.chunk_tx.take();
        }
    }

    /// Pull the next chunk within `timeout_ms`. Returns:
    /// - `Chunk` if a chunk was available
    /// - `End` (one-shot) if the lease has ended but End hasn't been delivered
    /// - `Timeout` if no chunk arrived in time and the lease is still open
    /// - `LeaseGone` if the lease was closed or End was already delivered
    pub async fn pull(
        &self,
        lease_id: &str,
        timeout_ms: u64,
    ) -> StreamPullResult {
        let lease = {
            let leases = self.leases.lock().await;
            match leases.get(lease_id) {
                Some(l) => l.clone(),
                None => return StreamPullResult::LeaseGone,
            }
        };

        // First check: has the stream already ended?
        {
            let mut g = lease.lock().await;
            if let Some(reason) = g.end_reason {
                if !g.end_delivered {
                    g.end_delivered = true;
                    return StreamPullResult::End {
                        reason,
                        error: g.end_error.clone(),
                    };
                }
                return StreamPullResult::LeaseGone;
            }
        }

        // Try to receive a chunk within the timeout.
        let chunk_opt = {
            let mut g = lease.lock().await;
            if timeout_ms == 0 {
                g.chunk_rx.try_recv().ok()
            } else {
                match timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    g.chunk_rx.recv(),
                )
                .await
                {
                    Ok(c) => c,
                    Err(_) => None,
                }
            }
        };

        match chunk_opt {
            Some(chunk) => StreamPullResult::Chunk { chunk },
            None => {
                // Re-check end state — End might have arrived while we waited.
                let mut g = lease.lock().await;
                if let Some(reason) = g.end_reason {
                    if !g.end_delivered {
                        g.end_delivered = true;
                        return StreamPullResult::End {
                            reason,
                            error: g.end_error.clone(),
                        };
                    }
                    return StreamPullResult::LeaseGone;
                }
                StreamPullResult::Timeout
            }
        }
    }

    /// Trigger cancellation. Idempotent.
    pub async fn cancel(&self, lease_id: &str) -> bool {
        let lease = {
            let leases = self.leases.lock().await;
            match leases.get(lease_id) {
                Some(l) => l.clone(),
                None => return false,
            }
        };
        let g = lease.lock().await;
        let _ = g.cancel_tx.send(true);
        debug!(lease_id, "Stream lease cancelled");
        true
    }

    /// Close and remove the lease. Aborts the pump task if still attached.
    /// Idempotent — closing a non-existent lease returns false but does not
    /// error.
    pub async fn close(&self, lease_id: &str) -> bool {
        let removed = self.leases.lock().await.remove(lease_id);
        if let Some(lease) = removed {
            let mut g = lease.lock().await;
            // Clean up the session_id index.
            if let Some(sid) = g.session_id.take() {
                self.by_session.lock().await.remove(&sid);
            }
            let _ = g.cancel_tx.send(true);
            if let Some(handle) = g.pump_handle.take() {
                handle.abort();
            }
            debug!(lease_id, "Stream lease closed");
            true
        } else {
            warn!(lease_id, "Stream close: lease not found (already gone?)");
            false
        }
    }

    /// Number of active leases (for diagnostics / shutdown cleanup).
    pub async fn len(&self) -> usize {
        self.leases.lock().await.len()
    }

    /// Close all leases (used during shutdown). Returns how many were closed.
    pub async fn close_all(&self) -> usize {
        let mut leases = self.leases.lock().await;
        let n = leases.len();
        for (_, lease) in leases.drain() {
            let mut g = lease.lock().await;
            let _ = g.cancel_tx.send(true);
            if let Some(h) = g.pump_handle.take() {
                h.abort();
            }
        }
        n
    }
}

// ============================================================================
// Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_registry() -> StreamLeaseRegistry {
        StreamLeaseRegistry::new()
    }

    #[tokio::test]
    async fn pull_from_empty_lease_times_out() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        let result = reg.pull("lease-1", 50).await;
        assert!(
            matches!(result, StreamPullResult::Timeout),
            "expected Timeout, got {result:?}"
        );
    }

    #[tokio::test]
    async fn push_then_pull_returns_chunk() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        let payload = StreamChunkPayload::Json(json!({"token": "hi"}));
        assert!(reg.route_chunk("lease-1", payload.clone()).await);

        let result = reg.pull("lease-1", 200).await;
        match result {
            StreamPullResult::Chunk { chunk } => {
                assert_eq!(chunk, payload);
            }
            other => panic!("expected Chunk, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn close_makes_subsequent_pull_return_lease_gone() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        assert!(reg.close("lease-1").await);

        let result = reg.pull("lease-1", 50).await;
        assert!(
            matches!(result, StreamPullResult::LeaseGone),
            "expected LeaseGone after close, got {result:?}"
        );
    }

    #[tokio::test]
    async fn signal_end_returns_end_once_then_lease_gone() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        reg.signal_end("lease-1", StreamEndReason::Completed, None)
            .await;

        // First pull after End returns End.
        let first = reg.pull("lease-1", 50).await;
        assert!(matches!(
            first,
            StreamPullResult::End {
                reason: StreamEndReason::Completed,
                error: None,
            }
        ));

        // Subsequent pulls return LeaseGone.
        let second = reg.pull("lease-1", 50).await;
        assert!(matches!(second, StreamPullResult::LeaseGone));
    }

    #[tokio::test]
    async fn cancel_flips_cancel_signal() {
        let reg = make_registry();
        let lease = reg.open("lease-1".into(), 4).await;
        assert!(!lease.lock().await.is_cancelled());
        assert!(reg.cancel("lease-1").await);
        assert!(lease.lock().await.is_cancelled());
    }

    #[tokio::test]
    async fn cancel_returns_false_for_missing_lease() {
        let reg = make_registry();
        assert!(!reg.cancel("nope").await);
    }

    #[tokio::test]
    async fn close_aborts_pump_handle() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;

        // Spawn a no-op pump task and attach it.
        let handle = tokio::spawn(async {});
        reg.attach_pump("lease-1", handle).await;

        // Closing should not panic.
        assert!(reg.close("lease-1").await);
        assert_eq!(reg.len().await, 0);
    }

    #[tokio::test]
    async fn end_can_be_signaled_with_error() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        reg.signal_end(
            "lease-1",
            StreamEndReason::Error,
            Some("boom".into()),
        )
        .await;

        let result = reg.pull("lease-1", 50).await;
        match result {
            StreamPullResult::End {
                reason: StreamEndReason::Error,
                error,
            } => assert_eq!(error.as_deref(), Some("boom")),
            other => panic!("expected End with Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn end_overwrite_does_not_replace_first_reason() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        reg.signal_end("lease-1", StreamEndReason::Cancelled, None)
            .await;
        // Late completion signal — should NOT overwrite Cancelled.
        reg.signal_end("lease-1", StreamEndReason::Completed, None)
            .await;

        let result = reg.pull("lease-1", 50).await;
        assert!(matches!(
            result,
            StreamPullResult::End {
                reason: StreamEndReason::Cancelled,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn pull_returns_end_even_if_chunk_pending_after_signal() {
        let reg = make_registry();
        reg.open("lease-1".into(), 4).await;
        let payload = StreamChunkPayload::Json(json!({"token": "first"}));
        reg.route_chunk("lease-1", payload).await;

        // After End is signaled, the chunk is still in the queue but End
        // takes precedence so the consumer learns the stream terminated
        // rather than receiving stale chunks.
        reg.signal_end("lease-1", StreamEndReason::Completed, None)
            .await;
        let result = reg.pull("lease-1", 50).await;
        assert!(matches!(result, StreamPullResult::End { .. }));
    }

    // ========================================================================
    // SHM ring fast-path tests (B.1) — Unix + shm-ring feature only.
    // ========================================================================
    #[cfg(all(unix, feature = "shm-ring"))]
    mod shm {
        use super::*;
        use neomind_extension_sdk::shm_ring::RingHandle;
        use std::sync::Arc;

        fn unique_name(label: &str) -> String {
            use std::sync::atomic::{AtomicU64, Ordering};
            static N: AtomicU64 = AtomicU64::new(0);
            let pid = std::process::id();
            let n = N.fetch_add(1, Ordering::SeqCst);
            format!("/nm-strmtest-{label}-{pid}-{n}")
        }

        #[tokio::test]
        async fn route_by_session_writes_to_ring_when_attached() {
            let reg = make_registry();
            reg.open("lease-shm".into(), 4).await;
            reg.set_session_id("lease-shm", "sess-1").await;

            // Create a ring and attach.
            let name = unique_name("rt");
            let ring = Arc::new(
                RingHandle::create(&name, 256, 4, RingDropPolicy::DropOldest).unwrap(),
            );
            reg.attach_ring("lease-shm", ring.clone()).await;

            // Route a chunk — should land in the ring (writer side).
            let chunk = json!({"token": "hello"});
            let matched = reg.route_by_session("sess-1", chunk).await;
            assert_eq!(matched, vec!["lease-shm".to_string()]);

            // Reader side: read it back.
            let reader = ring.reader();
            let mut buf = [0u8; 256];
            let frame = reader
                .try_read_for(std::time::Duration::from_millis(500), &mut buf)
                .expect("frame should be available");
            let s = std::str::from_utf8(frame.buf).unwrap();
            assert!(s.contains("hello"), "payload missing hello: {s}");
        }

        #[tokio::test]
        async fn route_by_session_falls_back_to_mpsc_without_ring() {
            // Without attach_ring, chunks go through mpsc → pull returns Chunk.
            let reg = make_registry();
            reg.open("lease-no-ring".into(), 4).await;
            reg.set_session_id("lease-no-ring", "sess-2").await;

            let matched = reg.route_by_session("sess-2", json!({"token": "x"})).await;
            assert_eq!(matched, vec!["lease-no-ring".to_string()]);

            let result = reg.pull("lease-no-ring", 200).await;
            assert!(matches!(result, StreamPullResult::Chunk { .. }));
        }
    }
}
