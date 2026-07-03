//! Stage 2 / A.4 — Stream API end-to-end tests.
//!
//! Two test layers:
//!
//! 1. **Wire-format closed loop** (always run) — drives every `IpcMessage`
//!    stream variant through `serde_json` roundtrip and validates that
//!    `StreamOpened` / `Success(StreamPullResult)` responses deserialize
//!    into the typed SDK structs the host side parses. This is the contract
//!    between runner and host — if it compiles and roundtrips, the protocol
//!    is closed.
//!
//! 2. **Full-process e2e** (`#[ignore]`) — spawns the runner subprocess with
//!    the smoke extension and a real `chat_stream` capability provider that
//!    publishes `AgentStreamChunk` events. Validated end-to-end through the
//!    actual IPC pipes. Gated because it requires:
//!      - a built `neomind-extension-runner` binary on PATH
//!      - a built `libneomind_smoke_extension.{dylib,so,dll}`
//!      - a chat_stream capability provider wired into the manager
//!    Enable with `cargo test -- --ignored stream_e2e_full`.

use neomind_extension_sdk::ipc::{
    IpcMessage, IpcResponse, StreamChunkPayload, StreamDropPolicy, StreamEndReason,
    StreamTransport, StreamTransportInfo,
};
use neomind_extension_sdk::{StreamOpenedInfo, StreamPullResult};
use serde_json::{json, Value};

// ============================================================================
// Layer 1 — Wire-format closed loop (always runs)
// ============================================================================

#[test]
fn stream_open_request_roundtrips_with_request_id() {
    let msg = IpcMessage::StreamOpen {
        request_id: 42,
        capability: "chat_stream".into(),
        params: json!({"message": "hi"}),
        buffer_size: 8,
        drop_policy: StreamDropPolicy::Block,
        transport: StreamTransport::IpcJson,
    };
    let wire = serde_json::to_vec(&msg).unwrap();
    let back: IpcMessage = serde_json::from_slice(&wire).unwrap();
    match back {
        IpcMessage::StreamOpen {
            request_id,
            capability,
            buffer_size,
            transport,
            ..
        } => {
            assert_eq!(request_id, 42);
            assert_eq!(capability, "chat_stream");
            assert_eq!(buffer_size, 8);
            assert_eq!(transport, StreamTransport::IpcJson);
        }
        other => panic!("expected StreamOpen, got {other:?}"),
    }
}

#[test]
fn stream_opened_response_parses_into_typed_info() {
    let response = IpcResponse::StreamOpened {
        request_id: 42,
        lease_id: "stream-abc".into(),
        initial_metadata: json!({"session_id": 7}),
        transport: StreamTransportInfo::IpcJson,
    };
    let wire = serde_json::to_vec(&response).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
    match parsed {
        IpcResponse::StreamOpened {
            request_id,
            lease_id,
            initial_metadata,
            transport,
        } => {
            assert_eq!(request_id, 42);
            // Construct the typed StreamOpenedInfo the way the host does.
            let info = StreamOpenedInfo {
                lease_id,
                initial_metadata,
                transport,
            };
            assert_eq!(info.lease_id, "stream-abc");
            assert_eq!(info.initial_metadata["session_id"], 7);
            assert_eq!(info.transport, StreamTransportInfo::IpcJson);
        }
        other => panic!("expected StreamOpened, got {other:?}"),
    }
}

#[test]
fn stream_pull_response_chunk_roundtrips() {
    // Runner wraps the pull result in IpcResponse::Success { data }.
    let pull_result = StreamPullResult::Chunk {
        chunk: StreamChunkPayload::Json(json!({"type": "token", "text": "hi"})),
    };
    let data = serde_json::to_value(&pull_result).unwrap();
    let response = IpcResponse::Success {
        request_id: 99,
        data: data,
    };

    let wire = serde_json::to_vec(&response).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
    match parsed {
        IpcResponse::Success { request_id, data } => {
            assert_eq!(request_id, 99);
            // Host deserializes data back into StreamPullResult.
            let back: StreamPullResult = serde_json::from_value(data).unwrap();
            match back {
                StreamPullResult::Chunk { chunk } => {
                    match chunk {
                        StreamChunkPayload::Json(v) => assert_eq!(v["text"], "hi"),
                        other => panic!("expected Json chunk, got {other:?}"),
                    }
                }
                other => panic!("expected Chunk, got {other:?}"),
            }
        }
        other => panic!("expected Success, got {other:?}"),
    }
}

#[test]
fn stream_pull_response_end_roundtrips_with_reason() {
    let pull_result = StreamPullResult::End {
        reason: StreamEndReason::Completed,
        error: None,
    };
    let data = serde_json::to_value(&pull_result).unwrap();
    let response = IpcResponse::Success {
        request_id: 100,
        data: data,
    };
    let wire = serde_json::to_vec(&response).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    let back: StreamPullResult = serde_json::from_value(data).unwrap();
    assert!(matches!(
        back,
        StreamPullResult::End {
            reason: StreamEndReason::Completed,
            error: None,
        }
    ));
}

#[test]
fn stream_pull_response_timeout_roundtrips() {
    let pull_result = StreamPullResult::Timeout;
    let data = serde_json::to_value(&pull_result).unwrap();
    let response = IpcResponse::Success {
        request_id: 1,
        data: data,
    };
    let wire = serde_json::to_vec(&response).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    let back: StreamPullResult = serde_json::from_value(data).unwrap();
    assert!(matches!(back, StreamPullResult::Timeout));
}

#[test]
fn stream_pull_response_lease_gone_roundtrips() {
    let pull_result = StreamPullResult::LeaseGone;
    let data = serde_json::to_value(&pull_result).unwrap();
    let response = IpcResponse::Success {
        request_id: 2,
        data: data,
    };
    let wire = serde_json::to_vec(&response).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    let back: StreamPullResult = serde_json::from_value(data).unwrap();
    assert!(matches!(back, StreamPullResult::LeaseGone));
}

#[test]
fn stream_pull_request_carries_request_id_for_correlation() {
    let msg = IpcMessage::StreamPull {
        request_id: 77,
        lease_id: "stream-xyz".into(),
        timeout_ms: 5000,
    };
    let wire = serde_json::to_vec(&msg).unwrap();
    let back: IpcMessage = serde_json::from_slice(&wire).unwrap();
    match back {
        IpcMessage::StreamPull {
            request_id,
            lease_id,
            timeout_ms,
        } => {
            assert_eq!(request_id, 77);
            assert_eq!(lease_id, "stream-xyz");
            assert_eq!(timeout_ms, 5000);
        }
        other => panic!("expected StreamPull, got {other:?}"),
    }
}

#[test]
fn stream_cancel_and_close_request_roundtrips() {
    let cancel = IpcMessage::StreamCancel {
        request_id: 1,
        lease_id: "L".into(),
        reason: "barge_in".into(),
    };
    let close = IpcMessage::StreamClose {
        request_id: 2,
        lease_id: "L".into(),
    };
    for (msg, name) in [
        (serde_json::to_vec(&cancel).unwrap(), "cancel"),
        (serde_json::to_vec(&close).unwrap(), "close"),
    ] {
        let _: IpcMessage = serde_json::from_slice(&msg).unwrap_or_else(|e| {
            panic!("failed to roundtrip {name} request: {e}");
        });
    }
}

#[test]
fn stream_cancel_close_success_responses_carry_boolean_payload() {
    let cancel_ack = IpcResponse::Success {
        request_id: 1,
        data: json!({"cancelled": true}),
    };
    let close_ack = IpcResponse::Success {
        request_id: 2,
        data: json!({"closed": true}),
    };

    for (resp, field, expected) in [
        (cancel_ack, "cancelled", true),
        (close_ack, "closed", true),
    ] {
        let wire = serde_json::to_vec(&resp).unwrap();
        let parsed: IpcResponse = serde_json::from_slice(&wire).unwrap();
        let IpcResponse::Success { data, .. } = parsed else {
            panic!("expected Success");
        };
        assert_eq!(data[field], expected);
    }
}

#[test]
fn full_closed_loop_simulation_local() {
    // Simulate the full closed loop on a single thread, exercising the
    // serialization layer exactly the way the runner and host do:
    //
    //   host: StreamOpen          ── wire ──▶  runner
    //   runner: StreamOpened      ◀── wire ──   runner
    //   host: StreamPull          ── wire ──▶  runner
    //   runner: Success(Chunk)    ◀── wire ──   runner
    //   runner: Success(End)      ◀── wire ──   runner
    //   host: StreamClose         ── wire ──▶  runner
    //   runner: Success(closed)   ◀── wire ──   runner

    let lease_id = "stream-sim-1";

    // 1. host → runner: StreamOpen
    let open = IpcMessage::StreamOpen {
        request_id: 10,
        capability: "chat_stream".into(),
        params: json!({"message": "hello"}),
        buffer_size: 4,
        drop_policy: StreamDropPolicy::Block,
        transport: StreamTransport::IpcJson,
    };
    let open_wire = serde_json::to_vec(&open).unwrap();
    let _: IpcMessage = serde_json::from_slice(&open_wire).unwrap();

    // 2. runner → host: StreamOpened
    let opened = IpcResponse::StreamOpened {
        request_id: 10,
        lease_id: lease_id.into(),
        initial_metadata: json!({}),
        transport: StreamTransportInfo::IpcJson,
    };
    let opened_wire = serde_json::to_vec(&opened).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&opened_wire).unwrap();
    let IpcResponse::StreamOpened {
        lease_id: parsed_lease,
        ..
    } = parsed
    else {
        panic!("expected StreamOpened");
    };
    assert_eq!(parsed_lease, lease_id);

    // 3. host → runner: StreamPull (returns Chunk)
    let _pull_req = IpcMessage::StreamPull {
        request_id: 11,
        lease_id: lease_id.into(),
        timeout_ms: 1000,
    };
    let chunk_resp = IpcResponse::Success {
        request_id: 11,
        data: serde_json::to_value(&StreamPullResult::Chunk {
            chunk: StreamChunkPayload::Text("tok".into()),
        })
        .unwrap(),
    };
    let chunk_wire = serde_json::to_vec(&chunk_resp).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&chunk_wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    let pr: StreamPullResult = serde_json::from_value(data).unwrap();
    assert!(matches!(pr, StreamPullResult::Chunk { .. }));

    // 4. runner → host: Success(End { Completed })
    let end_resp = IpcResponse::Success {
        request_id: 12,
        data: serde_json::to_value(&StreamPullResult::End {
            reason: StreamEndReason::Completed,
            error: None,
        })
        .unwrap(),
    };
    let end_wire = serde_json::to_vec(&end_resp).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&end_wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    let pr: StreamPullResult = serde_json::from_value(data).unwrap();
    assert!(matches!(
        pr,
        StreamPullResult::End {
            reason: StreamEndReason::Completed,
            ..
        }
    ));

    // 5. host → runner: StreamClose
    let close_req = IpcMessage::StreamClose {
        request_id: 13,
        lease_id: lease_id.into(),
    };
    let close_wire = serde_json::to_vec(&close_req).unwrap();
    let _: IpcMessage = serde_json::from_slice(&close_wire).unwrap();

    let close_resp = IpcResponse::Success {
        request_id: 13,
        data: json!({"closed": true}),
    };
    let close_resp_wire = serde_json::to_vec(&close_resp).unwrap();
    let parsed: IpcResponse = serde_json::from_slice(&close_resp_wire).unwrap();
    let IpcResponse::Success { data, .. } = parsed else {
        panic!("expected Success");
    };
    assert_eq!(data["closed"], true);
}

// ============================================================================
// Layer 2 — Full-process e2e (#[ignore])
// ============================================================================
//
// This test is gated because it needs:
//   - built neomind-extension-runner
//   - built smoke extension
//   - a chat_stream capability provider wired into IsolatedExtensionManager
//
// The infrastructure for the third item (string-keyed capability providers
// like chat_stream) lives in neomind-api, not neomind-core. Wiring a mock
// chat_stream provider here would duplicate that. Once the platform side
// exposes a test-friendly chat_stream mock, this test can be enabled.

#[tokio::test]
#[ignore = "requires built runner binary + smoke extension + chat_stream provider wiring"]
async fn stream_full_process_closed_loop() {
    use neomind_core::extension::isolated::{
        IsolatedExtension, IsolatedExtensionConfig, IsolatedExtensionError,
    };
    use std::path::PathBuf;
    use std::process::Command;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn runner_dir() -> PathBuf {
        workspace_root().join("target").join("debug")
    }

    fn smoke_extension_path() -> PathBuf {
        let lib_name = if cfg!(target_os = "macos") {
            "libneomind_smoke_extension.dylib"
        } else if cfg!(target_os = "windows") {
            "neomind_smoke_extension.dll"
        } else {
            "libneomind_smoke_extension.so"
        };
        runner_dir().join(lib_name)
    }

    // Build the runner + smoke extension.
    let status = Command::new("cargo")
        .current_dir(workspace_root())
        .args([
            "build",
            "-p",
            "neomind-extension-runner",
            "-p",
            "neomind-smoke-extension",
        ])
        .status()
        .expect("cargo build failed");
    assert!(status.success(), "build failed");

    // Put runner on PATH so IsolatedExtension can spawn it.
    let original_path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![runner_dir()];
    paths.extend(std::env::split_paths(&original_path));
    let joined = std::env::join_paths(paths).expect("join PATH");
    unsafe {
        std::env::set_var("PATH", &joined);
    }

    let path = smoke_extension_path();
    assert!(path.exists(), "smoke extension missing");

    let config = IsolatedExtensionConfig::default();
    let isolated = IsolatedExtension::new("smoke-stream-e2e", path, config);
    isolated.start().await.expect("start failed");

    // Open a stream.
    let info = isolated
        .stream_open(
            "chat_stream",
            &json!({"message": "hi"}),
            8,
            StreamDropPolicy::Block,
            StreamTransport::IpcJson,
        )
        .await
        .expect("stream_open failed");

    let lease_id = info.lease_id.clone();
    assert!(!lease_id.is_empty());

    // Pull until End.
    let mut got_chunk = false;
    let mut got_end = false;
    for _ in 0..64 {
        let result = isolated
            .stream_pull(&lease_id, 5_000)
            .await
            .expect("stream_pull failed");
        match result {
            StreamPullResult::Chunk { .. } => {
                got_chunk = true;
            }
            StreamPullResult::End {
                reason: StreamEndReason::Completed,
                ..
            } => {
                got_end = true;
                break;
            }
            StreamPullResult::Timeout => continue,
            other => panic!("unexpected pull result: {other:?}"),
        }
    }
    assert!(got_chunk, "expected at least one Chunk");
    assert!(got_end, "expected terminal End with Completed reason");

    let closed = isolated
        .stream_close(&lease_id)
        .await
        .expect("stream_close failed");
    assert!(closed);

    let _ = isolated.stop().await;
    let _ = original_path; // restore happens when test exits
}

// ============================================================================
// Layer 3 — Real-runner IPC integration (always runs)
//
// Spawns the actual runner process + smoke extension and drives the stream
// IPC protocol through real pipes. The "happy path" with a live chat_stream
// provider is the ignored test above; here we exercise the error / cleanup
// paths that the runner must handle correctly even without a provider.
// ============================================================================

mod real_runner {
    use super::*;
    use neomind_core::extension::isolated::{
        IsolatedExtension, IsolatedExtensionConfig,
    };
    use std::path::PathBuf;
    use std::process::Command;

    fn workspace_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn runner_dir() -> PathBuf {
        workspace_root().join("target").join("debug")
    }

    fn smoke_extension_path() -> PathBuf {
        let lib_name = if cfg!(target_os = "macos") {
            "libneomind_smoke_extension.dylib"
        } else if cfg!(target_os = "windows") {
            "neomind_smoke_extension.dll"
        } else {
            "libneomind_smoke_extension.so"
        };
        runner_dir().join(lib_name)
    }

    async fn spawn_isolated(id: &str) -> IsolatedExtension {
        // Build the runner + smoke extension if missing. The smoke extension's
        // build.rs sets `LC_ID_DYLIB=@rpath/extension.dylib` on macOS so the
        // runner's dylib validation accepts it.
        let status = Command::new("cargo")
            .current_dir(workspace_root())
            .args([
                "build",
                "-p",
                "neomind-extension-runner",
                "-p",
                "neomind-smoke-extension",
            ])
            .status()
            .expect("cargo build failed");
        assert!(status.success(), "build failed");

        let original_path = std::env::var_os("PATH").unwrap_or_default();
        let mut paths = vec![runner_dir()];
        paths.extend(std::env::split_paths(&original_path));
        let joined = std::env::join_paths(paths).expect("join PATH");
        unsafe {
            std::env::set_var("PATH", &joined);
        }

        let path = smoke_extension_path();
        assert!(path.exists(), "smoke extension missing: {path:?}");

        let config = IsolatedExtensionConfig::default();
        let isolated = IsolatedExtension::new(id, path, config);
        isolated.start().await.expect("runner start failed");
        isolated
    }

    /// Verifies that opening a stream for a capability with no host-side
    /// provider configured still completes the full StreamOpen → Pull → End
    /// loop cleanly. The runner must:
    ///   1. mint a lease and return `StreamOpened`
    ///   2. translate the capability-invocation error into `End::Error`
    ///   3. accept `stream_close` without leaking the lease
    #[tokio::test]
    async fn stream_open_pull_end_loop_when_no_provider_configured() {
        let isolated = spawn_isolated("smoke-no-provider").await;

        let info = isolated
            .stream_open(
                "chat_stream",
                &json!({"message": "hi"}),
                8,
                StreamDropPolicy::Block,
                StreamTransport::IpcJson,
            )
            .await
            .expect("stream_open IPC failed");

        let lease_id = info.lease_id.clone();
        assert!(!lease_id.is_empty(), "lease_id must be non-empty");
        assert!(
            matches!(info.transport, StreamTransportInfo::IpcJson),
            "expected IpcJson transport, got {:?}",
            info.transport
        );

        // First (and only) pull should be End::Error — the runner's pump
        // observes the capability invocation failure and signals end.
        let mut saw_terminal = false;
        for _ in 0..8 {
            let result = isolated
                .stream_pull(&lease_id, 5_000)
                .await
                .expect("stream_pull IPC failed");
            match result {
                StreamPullResult::End {
                    reason: StreamEndReason::Error,
                    error,
                } => {
                    assert!(
                        error.is_some(),
                        "End::Error must carry an error message"
                    );
                    saw_terminal = true;
                    break;
                }
                StreamPullResult::Timeout => continue,
                other => panic!("unexpected pull result: {other:?}"),
            }
        }
        assert!(saw_terminal, "expected terminal End::Error");

        // Close after End must still succeed (idempotent cleanup).
        let closed = isolated
            .stream_close(&lease_id)
            .await
            .expect("stream_close IPC failed");
        assert!(closed, "close must return true");

        let _ = isolated.stop().await;
    }

    /// Verifies the SHM-ring transport request path: host asks for
    /// `SharedMemRing`, runner creates a ring and returns the name in
    /// `StreamOpenedInfo::transport`. Falls back to IpcJson transparently if
    /// the runner can't create the ring.
    #[tokio::test]
    async fn stream_open_with_shm_ring_transport_returns_ring_info() {
        let isolated = spawn_isolated("smoke-shm-transport").await;

        let info = isolated
            .stream_open(
                "chat_stream",
                &json!({"message": "hi"}),
                8,
                StreamDropPolicy::Block,
                StreamTransport::SharedMemRing {
                    frame_size_max: 640,
                    frame_count_max: 64,
                },
            )
            .await
            .expect("stream_open IPC failed");

        // Either the runner created the ring (preferred) or downgraded to
        // IpcJson. Both are valid responses; the lease must work either way.
        match &info.transport {
            StreamTransportInfo::SharedMemRing { shm_name, .. } => {
                assert!(
                    !shm_name.is_empty(),
                    "shm_name must be non-empty when ring is created"
                );
            }
            StreamTransportInfo::IpcJson => {
                // Runner downgraded — also acceptable.
            }
        }

        // Drain to End regardless of transport.
        let lease_id = info.lease_id.clone();
        let mut saw_terminal = false;
        for _ in 0..8 {
            let result = isolated
                .stream_pull(&lease_id, 5_000)
                .await
                .expect("stream_pull IPC failed");
            if matches!(
                result,
                StreamPullResult::End {
                    reason: StreamEndReason::Error,
                    ..
                }
            ) {
                saw_terminal = true;
                break;
            }
        }
        assert!(saw_terminal, "expected terminal End::Error");

        let closed = isolated
            .stream_close(&lease_id)
            .await
            .expect("stream_close IPC failed");
        assert!(closed);

        let _ = isolated.stop().await;
    }

    /// Cancel + Close must not panic on a lease that already ended. Verifies
    /// the registry's lifecycle invariants across rapid cancel/close calls.
    #[tokio::test]
    async fn stream_cancel_and_close_after_end_are_safe() {
        let isolated = spawn_isolated("smoke-cancel-after-end").await;

        let info = isolated
            .stream_open(
                "chat_stream",
                &json!({"message": "hi"}),
                8,
                StreamDropPolicy::Block,
                StreamTransport::IpcJson,
            )
            .await
            .expect("stream_open IPC failed");

        let lease_id = info.lease_id.clone();

        // Drain to End.
        for _ in 0..8 {
            let result = isolated
                .stream_pull(&lease_id, 5_000)
                .await
                .expect("stream_pull IPC failed");
            if matches!(
                result,
                StreamPullResult::End {
                    reason: StreamEndReason::Error,
                    ..
                }
            ) {
                break;
            }
        }

        // Cancel after end — should be a no-op (returns true or false, must
        // not panic).
        let _ = isolated
            .stream_cancel(&lease_id, "user_aborted")
            .await
            .expect("stream_cancel IPC failed");

        // Close after end — returns true (lease existed) and cleans up.
        let closed = isolated
            .stream_close(&lease_id)
            .await
            .expect("stream_close IPC failed");
        assert!(closed);

        let _ = isolated.stop().await;
    }
}
