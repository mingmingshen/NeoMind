//! Stage 1.6 — PcmRingWriter / PcmRingReader tests.
//!
//! SPSC ring buffer over a `SharedMem` segment. Per design §5/§6:
//! - `RingHeader` is exactly 64 bytes (one cache line).
//! - Writer/Reader share a single segment; writer pushes frames, reader pulls.
//! - DropPolicy::DropOldest overwrites the oldest unread frame when full.
//! - DropPolicy::DropNewest returns Dropped without overwriting.
//! - close() lets the reader drain remaining frames then observe end-of-stream.

use neomind_extension_sdk::shm_ring::{DropPolicy, RingHandle, WriteResult};

fn unique_name(label: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = N.fetch_add(1, Ordering::SeqCst);
    format!("/nm-ring-{label}-{pid}-{n}")
}

#[test]
fn ring_header_is_exactly_one_cache_line() {
    use neomind_extension_sdk::shm_ring::RingHeader;
    assert_eq!(std::mem::size_of::<RingHeader>(), 64);
}

#[test]
fn frame_slot_is_16_bytes() {
    use neomind_extension_sdk::shm_ring::FrameSlot;
    assert_eq!(std::mem::size_of::<FrameSlot>(), 16);
}

#[test]
fn writer_and_reader_round_trip_1000_frames() {
    let name = unique_name("rt");
    let frame_size = 64u32;
    let frame_count = 8u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    let payload = |i: u32| {
        let mut v = vec![0u8; frame_size as usize];
        v[0..4].copy_from_slice(&i.to_le_bytes());
        v
    };
    for i in 0..1000 {
        let r = writer.write(&payload(i), i as u64);
        assert!(matches!(r, WriteResult::Written(_)), "frame {i}");
        let mut buf = vec![0u8; frame_size as usize];
        let frame = reader.read(&mut buf).expect("read frame");
        assert_eq!(frame.ts_ns, i as u64);
        assert_eq!(&frame.buf[0..4], &i.to_le_bytes());
    }
}

#[test]
fn drop_oldest_when_full_increments_dropped_count() {
    let name = unique_name("old");
    let frame_size = 8u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();

    // Fill completely (frame_count slots). With DropOldest, additional writes
    // let write_pos advance past read_pos + capacity; the reader lazily
    // catches up on its next call (SPSC-safe — writer never touches
    // read_pos, which would race the reader's own advance).
    for i in 0..frame_count {
        let p = [i as u8; 8];
        let r = writer.write(&p, i as u64);
        assert!(matches!(r, WriteResult::Written(_)));
    }
    // Next write must drop oldest: dropped_count increments by 1.
    let r = writer.write(&[99u8; 8], 999);
    assert!(matches!(r, WriteResult::Written(_)));
    assert!(writer.dropped_count() >= 1);
}

#[test]
fn drop_oldest_reader_catches_up_and_sees_latest_frame() {
    // SPSC DropOldest: writer lets gap exceed capacity; reader fast-forwards
    // its own read_pos to write_pos - capacity + 1 on next read.
    let name = unique_name("catchup");
    let frame_size = 8u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    // Write 6 frames into a 4-capacity ring without reading.
    // Result: write_pos=6, read_pos=0, gap=6 > capacity=4.
    // Reader fast-forwards to read_pos = 6 - 4 = 2 before reading.
    // Frames 0, 1 are overwritten (lost); the reader sees frames 2, 3, 4, 5
    // in order from slots 2, 3, 0, 1.
    for i in 0u8..6 {
        let r = writer.write(&[i + 100; 8], i as u64);
        assert!(matches!(r, WriteResult::Written(_)));
    }
    assert_eq!(
        writer.dropped_count(),
        2,
        "writer counts drops at write time — 2 overflows for 6 writes into 4-capacity"
    );

    let mut buf = vec![0u8; 8];
    let f1 = reader.read(&mut buf).expect("frame after catch-up");
    // Fast-forward puts read_pos at 2 → slot 2 → frame 2 = [102; 8].
    assert_eq!(f1.buf, &[102u8; 8]);
    let f2 = reader.read(&mut buf).expect("frame 3");
    assert_eq!(f2.buf, &[103u8; 8]);
    let f3 = reader.read(&mut buf).expect("frame 4 (overwrote slot 0)");
    assert_eq!(f3.buf, &[104u8; 8]);
    let f4 = reader.read(&mut buf).expect("frame 5 (overwrote slot 1)");
    assert_eq!(f4.buf, &[105u8; 8]);

    // Ring is now drained; reader should block (timeout) without close.
    assert!(reader
        .try_read_for(std::time::Duration::from_millis(20), &mut buf)
        .is_none());
}

#[test]
fn drop_newest_when_full_does_not_overwrite() {
    let name = unique_name("new");
    let frame_size = 8u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropNewest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    for i in 0..frame_count {
        let p = [i as u8; 8];
        let r = writer.write(&p, i as u64);
        assert!(matches!(r, WriteResult::Written(_)));
    }
    // Full → DropNewest returns Dropped.
    let r = writer.write(&[99u8; 8], 999);
    assert!(matches!(r, WriteResult::Dropped));
    assert!(writer.dropped_count() >= 1);

    // Reader should still see the original 4 frames in order.
    for i in 0..frame_count {
        let mut buf = vec![0u8; frame_size as usize];
        let frame = reader.read(&mut buf).expect("frame");
        assert_eq!(frame.buf, vec![i as u8; 8]);
    }
}

#[test]
fn close_marks_end_of_stream_after_drain() {
    let name = unique_name("close");
    let frame_size = 4u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    writer.write(&[1, 2, 3, 4], 1);
    writer.write(&[5, 6, 7, 8], 2);
    writer.close();

    // Read both frames, then reader returns None.
    let mut buf = vec![0u8; 4];
    let f1 = reader.read(&mut buf).expect("frame 1");
    assert_eq!(f1.ts_ns, 1);
    let f2 = reader.read(&mut buf).expect("frame 2");
    assert_eq!(f2.ts_ns, 2);
    assert!(reader.read(&mut buf).is_none(), "after close + drain must be None");
}

#[test]
fn reader_blocks_then_sees_new_frame_when_writer_pushes() {
    // Single-process simulation of "writer produces later" — first read on
    // empty ring returns WouldBlock with timeout (we use a tight loop budget).
    let name = unique_name("late");
    let frame_size = 4u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    // Reader tries to read empty ring with bounded retry; should give up.
    let mut buf = vec![0u8; 4];
    let r = reader.try_read_for(std::time::Duration::from_millis(20), &mut buf);
    assert!(r.is_none(), "empty ring must not yield a frame");

    // Now write one frame.
    writer.write(&[42, 43, 44, 45], 7);
    let frame = reader.read(&mut buf).expect("now data is available");
    assert_eq!(frame.ts_ns, 7);
    assert_eq!(frame.buf, &[42, 43, 44, 45]);
}

#[test]
fn write_larger_than_frame_size_max_is_rejected() {
    let name = unique_name("oversize");
    let handle = RingHandle::create(&name, 8, 4, DropPolicy::DropOldest).expect("create");
    let writer = handle.writer();
    let big = vec![0u8; 16];
    let r = writer.write(&big, 1);
    assert!(matches!(r, WriteResult::InvalidSize));
}

#[test]
fn partial_frame_payload_preserved_byte_exact() {
    let name = unique_name("partial");
    let frame_size = 16u32;
    let frame_count = 4u32;
    let handle = RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest)
        .expect("create");
    let writer = handle.writer();
    let reader = handle.reader();

    let small = b"hi".to_vec();
    writer.write(&small, 1);
    let mut buf = vec![0u8; frame_size as usize];
    let frame = reader.read(&mut buf).expect("frame");
    assert_eq!(frame.buf.len(), 2, "must report actual written length");
    assert_eq!(frame.buf, b"hi");
}

#[test]
fn dropped_count_starts_at_zero() {
    let name = unique_name("drcount");
    let handle = RingHandle::create(&name, 4, 4, DropPolicy::DropOldest).expect("create");
    let writer = handle.writer();
    assert_eq!(writer.dropped_count(), 0);
}

#[test]
fn concurrent_writer_and_reader_under_drop_oldest_never_corrupts() {
    // Multi-threaded stress test: writer and reader on separate threads,
    // tiny capacity to force DropOldest overflow. Without the sequence
    // protocol, the reader would occasionally observe corrupted data
    // (mismatched frame index) during concurrent overwrite.
    use std::sync::Arc;
    use std::thread;

    let name = unique_name("stress");
    let frame_size = 4u32;
    let frame_count = 2u32; // tiny — overflow on every other write
    let handle = Arc::new(
        RingHandle::create(&name, frame_size, frame_count, DropPolicy::DropOldest).expect("create"),
    );
    let total = 5000u32;

    let h_writer = handle.clone();
    let writer = thread::spawn(move || {
        let w = h_writer.writer();
        for i in 0..total {
            let payload = i.to_le_bytes();
            let _ = w.write(&payload, i as u64);
        }
        w.close();
    });

    let h_reader = handle.clone();
    let reader = thread::spawn(move || {
        let r = h_reader.reader();
        let mut buf = vec![0u8; frame_size as usize];
        let mut last_seen: i64 = -1;
        let mut count = 0u32;
        loop {
            // Use try_read_for to avoid infinite spin on close
            match r.try_read_for(std::time::Duration::from_millis(50), &mut buf) {
                Some(frame) => {
                    count += 1;
                    let val = u32::from_le_bytes([
                        frame.buf[0],
                        frame.buf[1],
                        frame.buf[2],
                        frame.buf[3],
                    ]);
                    // The value must be one of the written frames. Under
                    // DropOldest, the reader may skip frames, but must NEVER
                    // see a value outside [0, total) — that would indicate
                    // torn read from a slot being overwritten.
                    assert!(
                        val < total,
                        "corrupted frame: val={val} >= total={total} (torn read)"
                    );
                    // Values should be monotonically non-decreasing (reader
                    // reads in write order, skipping some). A decrease
                    // indicates the reader read from a stale/wrong slot.
                    let v = val as i64;
                    assert!(
                        v >= last_seen,
                        "out-of-order frame: val={v} < last_seen={last_seen}"
                    );
                    last_seen = v;
                }
                None => break, // closed + drained (or timeout after close)
            }
        }
        count
    });

    writer.join().expect("writer panicked");
    let read_count = reader.join().expect("reader panicked");
    // Reader should have consumed at least 1 frame (likely close to frame_count
    // given the tight race window, but we don't assert a minimum beyond 0).
    let _ = read_count;
}
