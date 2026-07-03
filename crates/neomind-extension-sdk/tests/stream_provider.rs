//! Stage 1.2 — ExtensionStreamProvider trait + StreamHandle tests.
//!
//! Validates that:
//! - The trait can be implemented by a mock provider
//! - open_stream returns a StreamHandle with a fresh lease_id, metadata, and rx
//! - When the driver pushes chunks, they arrive on `rx`
//! - When the provider's driver ends naturally, `join` resolves to `Completed`
//! - When the handle's cancel signal is triggered, `join` resolves to `Cancelled`
//! - StreamHandle::is_cancelled reflects the cancel state

#![cfg(test)]

use neomind_extension_sdk::ipc::{
    StreamChunkPayload, StreamDropPolicy, StreamEndReason, StreamTransport,
};
use neomind_extension_sdk::{ExtensionStreamProvider, HostStreamHandle as StreamHandle};
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// A mock provider that immediately pushes a fixed sequence of chunks then ends.
struct MockProvider {
    chunks: Vec<StreamChunkPayload>,
    initial_metadata: serde_json::Value,
}

#[async_trait::async_trait]
impl ExtensionStreamProvider for MockProvider {
    async fn open_stream(
        &self,
        _params: &serde_json::Value,
        buffer_size: u32,
        _drop_policy: StreamDropPolicy,
        _transport: StreamTransport,
    ) -> Result<StreamHandle, String> {
        let (tx, rx) = tokio::sync::mpsc::channel(buffer_size as usize);
        let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);

        let chunks: Vec<StreamChunkPayload> = self.chunks.iter().map(|c| match c {
            StreamChunkPayload::Binary(b) => StreamChunkPayload::Binary(b.clone()),
            StreamChunkPayload::Text(s) => StreamChunkPayload::Text(s.clone()),
            StreamChunkPayload::Json(v) => StreamChunkPayload::Json(v.clone()),
            StreamChunkPayload::EndOfStream => StreamChunkPayload::EndOfStream,
        }).collect();

        let join = tokio::spawn(async move {
            // Push each chunk; honor cancel
            for chunk in chunks {
                if *cancel_rx.borrow() {
                    return StreamEndReason::Cancelled;
                }
                if tx.send(chunk).await.is_err() {
                    return StreamEndReason::Completed; // consumer gone
                }
            }
            StreamEndReason::Completed
        });

        Ok(StreamHandle::new(
            uuid_like_id(),
            self.initial_metadata.clone(),
            rx,
            cancel_tx,
            join,
        ))
    }
}

fn uuid_like_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("lease-{nanos}")
}

#[tokio::test]
async fn open_stream_returns_handle_with_metadata_and_lease_id() {
    let provider = MockProvider {
        chunks: vec![],
        initial_metadata: json!({"session_id": "sess-1", "created": true}),
    };
    let handle = provider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .expect("open_stream");

    assert!(!handle.lease_id().is_empty(), "lease_id non-empty");
    assert_eq!(handle.initial_metadata()["session_id"], "sess-1");
    assert_eq!(handle.initial_metadata()["created"], true);
}

#[tokio::test]
async fn pushed_chunks_arrive_on_rx_in_order() {
    let provider = MockProvider {
        chunks: vec![
            StreamChunkPayload::Text("alpha".into()),
            StreamChunkPayload::Text("beta".into()),
            StreamChunkPayload::Binary(vec![1, 2, 3]),
        ],
        initial_metadata: json!({}),
    };
    let mut handle = provider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .unwrap();

    let first = handle.rx().recv().await.expect("first chunk");
    let second = handle.rx().recv().await.expect("second chunk");
    let third = handle.rx().recv().await.expect("third chunk");
    assert!(matches!(first, StreamChunkPayload::Text(s) if s == "alpha"));
    assert!(matches!(second, StreamChunkPayload::Text(s) if s == "beta"));
    assert!(matches!(third, StreamChunkPayload::Binary(b) if b == vec![1, 2, 3]));
}

#[tokio::test]
async fn join_returns_completed_when_driver_finishes_naturally() {
    let provider = MockProvider {
        chunks: vec![StreamChunkPayload::Text("hi".into())],
        initial_metadata: json!({}),
    };
    let mut handle = provider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .unwrap();

    // Drain the chunk so driver can finish
    let _ = handle.rx().recv().await;

    let reason = timeout(Duration::from_secs(2), handle.join_mut().unwrap())
        .await
        .expect("join did not resolve in 2s")
        .expect("join task panicked");
    assert_eq!(reason, StreamEndReason::Completed);
}

#[tokio::test]
async fn cancel_triggers_driver_to_return_cancelled() {
    // This provider pushes infinitely, so only cancel can stop it.
    struct InfiniteProvider;
    #[async_trait::async_trait]
    impl ExtensionStreamProvider for InfiniteProvider {
        async fn open_stream(
            &self,
            _params: &serde_json::Value,
            buffer_size: u32,
            _drop_policy: StreamDropPolicy,
            _transport: StreamTransport,
        ) -> Result<StreamHandle, String> {
            let (tx, rx) = tokio::sync::mpsc::channel(buffer_size as usize);
            let (cancel_tx, mut cancel_rx) = tokio::sync::watch::channel(false);

            let join = tokio::spawn(async move {
                let mut n = 0u32;
                loop {
                    tokio::select! {
                        _ = cancel_rx.changed() => {
                            return StreamEndReason::Cancelled;
                        }
                        _ = tx.send(StreamChunkPayload::Binary(vec![n as u8])), if !*cancel_rx.borrow() => {
                            n += 1;
                        }
                    }
                }
            });

            Ok(StreamHandle::new(
                uuid_like_id(),
                json!({}),
                rx,
                cancel_tx,
                join,
            ))
        }
    }

    let mut handle = InfiniteProvider
        .open_stream(&json!({}), 4, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .unwrap();

    // Fire cancel
    handle.cancel();

    let reason = timeout(Duration::from_secs(2), handle.join_mut().unwrap())
        .await
        .expect("join did not resolve in 2s after cancel")
        .expect("join task panicked");
    assert_eq!(reason, StreamEndReason::Cancelled);
}

#[tokio::test]
async fn is_cancelled_reflects_cancel_state() {
    let provider = MockProvider {
        chunks: vec![StreamChunkPayload::Text("hi".into())],
        initial_metadata: json!({}),
    };
    let handle = provider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .unwrap();
    assert!(!handle.is_cancelled());
    handle.cancel();
    assert!(handle.is_cancelled());
}

#[tokio::test]
async fn cancel_is_idempotent() {
    let provider = MockProvider {
        chunks: vec![],
        initial_metadata: json!({}),
    };
    let handle = provider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await
        .unwrap();
    handle.cancel();
    handle.cancel(); // second call must not panic
    assert!(handle.is_cancelled());
}

#[tokio::test]
async fn provider_can_return_error_via_open_stream() {
    struct ErroringProvider;
    #[async_trait::async_trait]
    impl ExtensionStreamProvider for ErroringProvider {
        async fn open_stream(
            &self,
            _params: &serde_json::Value,
            _buffer_size: u32,
            _drop_policy: StreamDropPolicy,
            _transport: StreamTransport,
        ) -> Result<StreamHandle, String> {
            Err("session not ready".to_string())
        }
    }
    let result = ErroringProvider
        .open_stream(&json!({}), 8, StreamDropPolicy::Block, StreamTransport::IpcJson)
        .await;
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "session not ready");
}
