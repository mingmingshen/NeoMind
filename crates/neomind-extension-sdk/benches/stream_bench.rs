//! Stage 2 — Performance microbenchmarks for the stream API hot paths.
//!
//! Run with:
//!   cargo bench -p neomind-extension-sdk --features shm-ring
//! or
//!   cargo test -p neomind-extension-sdk --features shm-ring --bench stream_bench -- --nocapture
//!
//! Measures three things that determine end-to-end stream throughput:
//!   1. SHM ring write/read throughput on PCM-sized frames
//!      (the fast path used for audio)
//!   2. mpsc channel throughput (the fallback used for IpcJson transport)
//!   3. StreamChunkPayload::Binary serialization overhead
//!      (base64 vs the legacy integer-array form)

#![cfg(all(unix, feature = "shm-ring"))]

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use neomind_extension_sdk::ipc::StreamChunkPayload;
use neomind_extension_sdk::shm_ring::{DropPolicy, RingHandle};

// ---------- 1. SHM ring throughput ----------

fn bench_shm_ring(c: &mut Criterion) {
    let mut group = c.benchmark_group("shm_ring");
    let frame_sizes: &[(usize, &str)] = &[
        (640, "640B_8kHz_mono_pcm"),    // 40ms @ 8kHz mono 16-bit
        (1920, "1920B_16kHz_mono_pcm"), // 60ms @ 16kHz mono 16-bit
        (3840, "3840B_16k_stereo"),     // 60ms @ 16kHz stereo 16-bit
    ];
    for (size, label) in frame_sizes {
        group.throughput(Throughput::Bytes(*size as u64));
        group.bench_with_input(BenchmarkId::new("write_read", label), size, |b, &size| {
            // Set up the ring once: 512 frames of `size` bytes each.
            let name = format!("/nm-bench-{}", std::process::id());
            let ring = RingHandle::create(&name, size as u32, 512, DropPolicy::DropOldest)
                .expect("create ring");
            let payload = vec![0xABu8; size];
            let reader_payload = vec![0u8; size];
            b.iter(|| {
                let writer = ring.writer();
                writer.write(black_box(&payload), 123);
                let reader = ring.reader();
                let mut buf = reader_payload.clone();
                let _ = reader.try_read_for(std::time::Duration::from_millis(100), &mut buf);
            });
            // RingHandle drops here → SharedMem::drop calls shm_unlink.
        });
    }
    group.finish();
}

// ---------- 2. mpsc channel throughput ----------

fn bench_mpsc(c: &mut Criterion) {
    let mut group = c.benchmark_group("mpsc_fallback");
    let sizes: &[usize] = &[640, 3840];
    for &size in sizes {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("send_recv", size), &size, |b, &size| {
            let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);
            let payload = vec![0xCDu8; size];
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.iter(|| {
                let p = payload.clone();
                tx.try_send(p).unwrap();
                rt.block_on(async { rx.recv().await.unwrap() });
            });
        });
    }
    group.finish();
}

// ---------- 3. Binary payload serialization ----------

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    // Base64 (current) path.
    for &size in &[640usize, 3840] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::new("binary_base64", size), &size, |b, &size| {
            let chunk = StreamChunkPayload::Binary(vec![0u8; size]);
            b.iter(|| {
                let s = serde_json::to_string(black_box(&chunk)).unwrap();
                let _: StreamChunkPayload = serde_json::from_str(&s).unwrap();
            });
        });
    }

    // Json variant for comparison.
    group.throughput(Throughput::Bytes(200));
    group.bench_function("json_200B", |b| {
        let chunk = StreamChunkPayload::Json(serde_json::json!({
            "type": "chat_chunk",
            "session_id": 12345,
            "chunk": {"delta": "hello world this is some content"}
        }));
        b.iter(|| {
            let s = serde_json::to_string(black_box(&chunk)).unwrap();
            let _: StreamChunkPayload = serde_json::from_str(&s).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_shm_ring, bench_mpsc, bench_serialization);
criterion_main!(benches);
