//! Stage 1.1 — IPC stream types tests.
//!
//! Validates serialization, variant matching, and roundtrip for the new
//! stream-related IPC types added to `ipc_types.rs`.
//!
//! These tests are written FIRST (TDD red). They fail to compile until
//! the new types/variants are added to `ipc_types.rs`.

#![cfg(test)]

use neomind_extension_sdk::ipc::{
    IpcMessage, IpcResponse, StreamChunkPayload, StreamDropPolicy, StreamEndReason,
    StreamTransport, StreamTransportInfo,
};
use serde_json::json;

// ============================================================================
// StreamDropPolicy
// ============================================================================

#[test]
fn stream_drop_policy_block_serializes_as_lowercase() {
    let bytes = serde_json::to_vec(&StreamDropPolicy::Block).unwrap();
    assert_eq!(bytes, b"\"block\"");
}

#[test]
fn stream_drop_policy_drop_oldest_serializes_as_snake_case() {
    let bytes = serde_json::to_vec(&StreamDropPolicy::DropOldest).unwrap();
    assert_eq!(bytes, b"\"drop_oldest\"");
}

#[test]
fn stream_drop_policy_drop_newest_roundtrip() {
    let policy = StreamDropPolicy::DropNewest;
    let s = serde_json::to_string(&policy).unwrap();
    let back: StreamDropPolicy = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamDropPolicy::DropNewest));
}

// ============================================================================
// StreamChunkPayload
// ============================================================================

#[test]
fn stream_chunk_payload_json_roundtrip_preserves_value() {
    let payload = StreamChunkPayload::Json(json!({"type": "content", "text": "hello"}));
    let bytes = serde_json::to_vec(&payload).unwrap();
    let back: StreamChunkPayload = serde_json::from_slice(&bytes).unwrap();
    match back {
        StreamChunkPayload::Json(v) => assert_eq!(v["text"], "hello"),
        _ => panic!("expected Json variant"),
    }
}

#[test]
fn stream_chunk_payload_binary_roundtrip_preserves_bytes() {
    let payload = StreamChunkPayload::Binary(vec![0u8, 1, 2, 3, 255]);
    let bytes = serde_json::to_vec(&payload).unwrap();
    let back: StreamChunkPayload = serde_json::from_slice(&bytes).unwrap();
    match back {
        StreamChunkPayload::Binary(b) => assert_eq!(b, vec![0u8, 1, 2, 3, 255]),
        _ => panic!("expected Binary variant"),
    }
}

#[test]
fn stream_chunk_payload_text_roundtrip_preserves_string() {
    let payload = StreamChunkPayload::Text("你好世界".to_string());
    let bytes = serde_json::to_vec(&payload).unwrap();
    let back: StreamChunkPayload = serde_json::from_slice(&bytes).unwrap();
    match back {
        StreamChunkPayload::Text(s) => assert_eq!(s, "你好世界"),
        _ => panic!("expected Text variant"),
    }
}

#[test]
fn stream_chunk_payload_end_of_stream_is_tagged() {
    let payload = StreamChunkPayload::EndOfStream;
    let s = serde_json::to_string(&payload).unwrap();
    assert!(s.contains("end_of_stream") || s.contains("EndOfStream"));
    let back: StreamChunkPayload = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamChunkPayload::EndOfStream));
}

// ============================================================================
// StreamEndReason
// ============================================================================

#[test]
fn stream_end_reason_completed_roundtrip() {
    let s = serde_json::to_string(&StreamEndReason::Completed).unwrap();
    let back: StreamEndReason = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamEndReason::Completed));
}

#[test]
fn stream_end_reason_cancelled_roundtrip() {
    let s = serde_json::to_string(&StreamEndReason::Cancelled).unwrap();
    let back: StreamEndReason = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamEndReason::Cancelled));
}

#[test]
fn stream_end_reason_error_roundtrip() {
    let s = serde_json::to_string(&StreamEndReason::Error).unwrap();
    let back: StreamEndReason = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamEndReason::Error));
}

#[test]
fn stream_end_reason_host_shutdown_roundtrip() {
    let s = serde_json::to_string(&StreamEndReason::HostShutdown).unwrap();
    let back: StreamEndReason = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamEndReason::HostShutdown));
}

#[test]
fn stream_end_reason_lease_expired_roundtrip() {
    let s = serde_json::to_string(&StreamEndReason::LeaseExpired).unwrap();
    let back: StreamEndReason = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamEndReason::LeaseExpired));
}

// ============================================================================
// StreamTransport & StreamTransportInfo
// ============================================================================

#[test]
fn stream_transport_ipc_json_tag() {
    let s = serde_json::to_string(&StreamTransport::IpcJson).unwrap();
    assert!(s.contains("ipc_json") || s.contains("IpcJson"));
}

#[test]
fn stream_transport_shared_mem_ring_roundtrip() {
    let t = StreamTransport::SharedMemRing {
        frame_size_max: 640,
        frame_count_max: 64,
    };
    let s = serde_json::to_string(&t).unwrap();
    let back: StreamTransport = serde_json::from_str(&s).unwrap();
    match back {
        StreamTransport::SharedMemRing {
            frame_size_max,
            frame_count_max,
        } => {
            assert_eq!(frame_size_max, 640);
            assert_eq!(frame_count_max, 64);
        }
        _ => panic!("expected SharedMemRing variant"),
    }
}

#[test]
fn stream_transport_info_ipc_json_has_no_shm_name() {
    let info = StreamTransportInfo::IpcJson;
    let s = serde_json::to_string(&info).unwrap();
    let back: StreamTransportInfo = serde_json::from_str(&s).unwrap();
    assert!(matches!(back, StreamTransportInfo::IpcJson));
}

#[test]
fn stream_transport_info_shared_mem_ring_roundtrip() {
    let info = StreamTransportInfo::SharedMemRing {
        shm_name: "/neomind-pcm-test-123".to_string(),
        frame_size_max: 640,
        frame_count_max: 64,
    };
    let s = serde_json::to_string(&info).unwrap();
    let back: StreamTransportInfo = serde_json::from_str(&s).unwrap();
    match back {
        StreamTransportInfo::SharedMemRing {
            shm_name,
            frame_size_max,
            frame_count_max,
        } => {
            assert_eq!(shm_name, "/neomind-pcm-test-123");
            assert_eq!(frame_size_max, 640);
            assert_eq!(frame_count_max, 64);
        }
        _ => panic!("expected SharedMemRing variant"),
    }
}

// ============================================================================
// IpcMessage stream variants — roundtrip via to_bytes / from_bytes
// ============================================================================

#[test]
fn ipc_message_stream_open_roundtrip_preserves_all_fields() {
    let msg = IpcMessage::StreamOpen {
        request_id: 42,
        capability: "chat_stream".to_string(),
        params: json!({"message": "hi"}),
        buffer_size: 32,
        drop_policy: StreamDropPolicy::Block,
        transport: StreamTransport::SharedMemRing {
            frame_size_max: 640,
            frame_count_max: 64,
        },
    };
    let bytes = msg.to_bytes().unwrap();
    let decoded = IpcMessage::from_bytes(&bytes).unwrap();
    match decoded {
        IpcMessage::StreamOpen {
            request_id,
            capability,
            params,
            buffer_size,
            drop_policy,
            transport,
        } => {
            assert_eq!(request_id, 42);
            assert_eq!(capability, "chat_stream");
            assert_eq!(params, json!({"message": "hi"}));
            assert_eq!(buffer_size, 32);
            assert!(matches!(drop_policy, StreamDropPolicy::Block));
            match transport {
                StreamTransport::SharedMemRing {
                    frame_size_max,
                    frame_count_max,
                } => {
                    assert_eq!(frame_size_max, 640);
                    assert_eq!(frame_count_max, 64);
                }
                _ => panic!("expected SharedMemRing transport"),
            }
        }
        _ => panic!("expected StreamOpen variant"),
    }
}

#[test]
fn ipc_message_stream_pull_roundtrip() {
    let msg = IpcMessage::StreamPull {
        lease_id: "lease-abc".to_string(),
        timeout_ms: 5000,
        request_id: 42,
    };
    let bytes = msg.to_bytes().unwrap();
    let decoded = IpcMessage::from_bytes(&bytes).unwrap();
    match decoded {
        IpcMessage::StreamPull { lease_id, timeout_ms, .. } => {
            assert_eq!(lease_id, "lease-abc");
            assert_eq!(timeout_ms, 5000);
        }
        _ => panic!("expected StreamPull variant"),
    }
}

#[test]
fn ipc_message_stream_cancel_roundtrip_preserves_reason() {
    let msg = IpcMessage::StreamCancel {
        lease_id: "lease-xyz".to_string(),
        reason: "barge_in".to_string(),
        request_id: 7,
    };
    let bytes = msg.to_bytes().unwrap();
    let decoded = IpcMessage::from_bytes(&bytes).unwrap();
    match decoded {
        IpcMessage::StreamCancel { lease_id, reason, .. } => {
            assert_eq!(lease_id, "lease-xyz");
            assert_eq!(reason, "barge_in");
        }
        _ => panic!("expected StreamCancel variant"),
    }
}

#[test]
fn ipc_message_stream_close_roundtrip() {
    let msg = IpcMessage::StreamClose {
        lease_id: "lease-end".to_string(),
        request_id: 99,
    };
    let bytes = msg.to_bytes().unwrap();
    let decoded = IpcMessage::from_bytes(&bytes).unwrap();
    match decoded {
        IpcMessage::StreamClose { lease_id, .. } => assert_eq!(lease_id, "lease-end"),
        _ => panic!("expected StreamClose variant"),
    }
}

// ============================================================================
// IpcResponse stream variants
// ============================================================================

#[test]
fn ipc_response_stream_opened_roundtrip_with_metadata() {
    let resp = IpcResponse::StreamOpened {
        request_id: 99,
        lease_id: "lease-new".to_string(),
        initial_metadata: json!({"session_id": "sess-1", "created": true}),
        transport: StreamTransportInfo::SharedMemRing {
            shm_name: "/neomind-pcm-abc".to_string(),
            frame_size_max: 640,
            frame_count_max: 64,
        },
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamOpened {
            request_id,
            lease_id,
            initial_metadata,
            transport,
        } => {
            assert_eq!(request_id, 99);
            assert_eq!(lease_id, "lease-new");
            assert_eq!(initial_metadata["session_id"], "sess-1");
            assert_eq!(initial_metadata["created"], true);
            match transport {
                StreamTransportInfo::SharedMemRing { shm_name, .. } => {
                    assert_eq!(shm_name, "/neomind-pcm-abc");
                }
                _ => panic!("expected SharedMemRing transport"),
            }
        }
        _ => panic!("expected StreamOpened variant"),
    }
}

#[test]
fn ipc_response_stream_chunk_binary_roundtrip() {
    let resp = IpcResponse::StreamChunk {
        lease_id: "lease-1".to_string(),
        chunk: StreamChunkPayload::Binary(vec![1, 2, 3]),
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamChunk { lease_id, chunk } => {
            assert_eq!(lease_id, "lease-1");
            match chunk {
                StreamChunkPayload::Binary(b) => assert_eq!(b, vec![1, 2, 3]),
                _ => panic!("expected Binary chunk"),
            }
        }
        _ => panic!("expected StreamChunk variant"),
    }
}

#[test]
fn ipc_response_stream_chunk_end_of_stream_roundtrip() {
    let resp = IpcResponse::StreamChunk {
        lease_id: "lease-end".to_string(),
        chunk: StreamChunkPayload::EndOfStream,
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamChunk { chunk, .. } => {
            assert!(matches!(chunk, StreamChunkPayload::EndOfStream));
        }
        _ => panic!("expected StreamChunk variant"),
    }
}

#[test]
fn ipc_response_stream_end_cancelled_with_error() {
    let resp = IpcResponse::StreamEnd {
        lease_id: "lease-x".to_string(),
        reason: StreamEndReason::Cancelled,
        error: Some("user barge-in".to_string()),
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamEnd { lease_id, reason, error } => {
            assert_eq!(lease_id, "lease-x");
            assert!(matches!(reason, StreamEndReason::Cancelled));
            assert_eq!(error.as_deref(), Some("user barge-in"));
        }
        _ => panic!("expected StreamEnd variant"),
    }
}

#[test]
fn ipc_response_stream_end_completed_no_error() {
    let resp = IpcResponse::StreamEnd {
        lease_id: "lease-done".to_string(),
        reason: StreamEndReason::Completed,
        error: None,
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamEnd { reason, error, .. } => {
            assert!(matches!(reason, StreamEndReason::Completed));
            assert!(error.is_none());
        }
        _ => panic!("expected StreamEnd variant"),
    }
}

#[test]
fn ipc_response_stream_cancel_ack_roundtrip() {
    let resp = IpcResponse::StreamCancelAck {
        lease_id: "lease-c".to_string(),
        cancelled: true,
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    match decoded {
        IpcResponse::StreamCancelAck { lease_id, cancelled } => {
            assert_eq!(lease_id, "lease-c");
            assert!(cancelled);
        }
        _ => panic!("expected StreamCancelAck variant"),
    }
}

// ============================================================================
// Discrimination: existing variants still work (regression guard)
// ============================================================================

#[test]
fn existing_ipc_message_init_still_serializes() {
    // Ensures adding new variants didn't break the existing enum tag scheme.
    let msg = IpcMessage::Init { config: json!({}) };
    let bytes = msg.to_bytes().unwrap();
    let decoded = IpcMessage::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, IpcMessage::Init { .. }));
}

#[test]
fn existing_ipc_response_success_still_serializes() {
    let resp = IpcResponse::Success {
        request_id: 1,
        data: json!({"ok": true}),
    };
    let bytes = resp.to_bytes().unwrap();
    let decoded = IpcResponse::from_bytes(&bytes).unwrap();
    assert!(matches!(decoded, IpcResponse::Success { .. }));
}
