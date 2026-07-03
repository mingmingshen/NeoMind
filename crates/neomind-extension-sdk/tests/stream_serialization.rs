//! Stage 2 / A.0 — Binary payload serialization tests (TDD red).
//!
//! Validates that `StreamChunkPayload::Binary` serializes as a base64 string
//! (compact, ~1.33x overhead) instead of a JSON integer array (~2.8x overhead
//! for typical PCM frames). The deserializer accepts BOTH the new base64 form
//! and the legacy integer-array form so Stage 1 fixtures stay valid.

#![cfg(test)]

use neomind_extension_sdk::ipc::StreamChunkPayload;
use serde_json::json;

// ============================================================================
// Binary variant — new base64 form
// ============================================================================

#[test]
fn binary_serializes_as_base64_string_not_array() {
    let payload = StreamChunkPayload::Binary(vec![1, 2, 3, 255, 0, 128]);
    let v: serde_json::Value = serde_json::to_value(&payload).unwrap();

    // Expected shape: {"Binary": "<base64>"} where <base64> is a STRING.
    let binary_field = v.get("Binary").expect("missing Binary key");
    assert!(
        binary_field.is_string(),
        "Binary payload should serialize as a base64 string, got: {binary_field}"
    );

    // Verify the base64 content roundtrips to the original bytes.
    let s = binary_field.as_str().unwrap();
    let decoded = base64_decode(s);
    assert_eq!(decoded, vec![1, 2, 3, 255, 0, 128]);
}

#[test]
fn binary_base64_is_smaller_than_array_for_640_bytes() {
    // 640 bytes = 16kHz/mono/16bit/20ms PCM frame.
    let pcm: Vec<u8> = (0..640).map(|i| (i % 256) as u8).collect();
    let payload = StreamChunkPayload::Binary(pcm.clone());

    let json = serde_json::to_vec(&payload).unwrap();
    // Base64 encoding: 640 -> 856 chars + small JSON wrapper.
    // Sanity bound: must be well under 2000 bytes (legacy array form ~2000+).
    assert!(
        json.len() < 1000,
        "base64-encoded 640-byte PCM should be < 1000 bytes, got {}",
        json.len()
    );
    assert!(
        json.len() > 856,
        "base64 payload should be at least ceil(640/3)*4 = 856 bytes, got {}",
        json.len()
    );

    // Also compare directly against the legacy array form.
    let legacy_array: serde_json::Value = json!(pcm);
    let legacy_bytes = serde_json::to_vec(&legacy_array).unwrap();
    assert!(
        json.len() < legacy_bytes.len(),
        "base64 form ({}) should be smaller than array form ({})",
        json.len(),
        legacy_bytes.len()
    );
}

// ============================================================================
// Binary variant — legacy array form (backward compat)
// ============================================================================

#[test]
fn binary_deserializes_legacy_array_form() {
    // The Stage 1 form serialized Binary as a JSON integer array.
    // Old fixtures / in-flight messages must still parse.
    let legacy = json!({"Binary": [1, 2, 3, 255]});
    let payload: StreamChunkPayload = serde_json::from_value(legacy).unwrap();
    assert_eq!(payload, StreamChunkPayload::Binary(vec![1, 2, 3, 255]));
}

#[test]
fn binary_deserializes_new_base64_form() {
    // 3 bytes -> "AAEC" base64 prefix (we don't pin the exact encoding here,
    // we just verify that a known base64 string roundtrips).
    let bytes = vec![0u8, 1, 2, 3, 255];
    let encoded = base64_encode(&bytes);
    let new_form = json!({"Binary": encoded});
    let payload: StreamChunkPayload = serde_json::from_value(new_form).unwrap();
    assert_eq!(payload, StreamChunkPayload::Binary(bytes));
}

// ============================================================================
// Other variants unchanged
// ============================================================================

#[test]
fn json_variant_unchanged() {
    let payload = StreamChunkPayload::Json(json!({"token": "hi"}));
    let v: serde_json::Value = serde_json::to_value(&payload).unwrap();
    assert_eq!(v, json!({"Json": {"token": "hi"}}));

    let back: StreamChunkPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back, payload);
}

#[test]
fn text_variant_unchanged() {
    let payload = StreamChunkPayload::Text("hello".to_string());
    let v: serde_json::Value = serde_json::to_value(&payload).unwrap();
    assert_eq!(v, json!({"Text": "hello"}));

    let back: StreamChunkPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back, payload);
}

#[test]
fn end_of_stream_variant_unchanged() {
    let payload = StreamChunkPayload::EndOfStream;
    let v: serde_json::Value = serde_json::to_value(&payload).unwrap();
    assert_eq!(v, json!("EndOfStream"));

    let back: StreamChunkPayload = serde_json::from_value(v).unwrap();
    assert_eq!(back, payload);
}

// ============================================================================
// Test helpers — minimal base64 codec so tests don't depend on the SDK's
// own base64 crate choice (we want to verify the wire format, not re-import
// the impl).
// ============================================================================

fn base64_encode(bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(bytes)
}

fn base64_decode(s: &str) -> Vec<u8> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.decode(s).unwrap()
}
