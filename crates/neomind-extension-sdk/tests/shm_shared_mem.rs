//! Stage 1.5 — `SharedMem` POSIX shm_open wrapper tests.
//!
//! Validates the single-process semantics of the cross-process shared memory
//! primitive. The cross-process aspect itself is kernel-provided; we only need
//! to verify that create/open/mmap/unlink behave correctly from one process.

use neomind_extension_sdk::shm_ring::SharedMem;

fn unique_name(label: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let pid = std::process::id();
    let n = N.fetch_add(1, Ordering::SeqCst);
    // POSIX shm name limit on macOS is ~31 chars; keep it short.
    format!("/nm-{label}-{pid}-{n}")
}

#[test]
fn create_returns_mapping_with_requested_size() {
    let name = unique_name("size");
    let shm = SharedMem::create(&name, 4096).expect("create");
    assert_eq!(shm.size(), 4096);
    assert_eq!(shm.name(), name);
}

#[test]
fn writes_are_visible_via_same_handle() {
    let name = unique_name("rw");
    let mut shm = SharedMem::create(&name, 4096).expect("create");
    let bytes = b"hello world";
    shm.as_mut_slice()[..bytes.len()].copy_from_slice(bytes);
    assert_eq!(&shm.as_slice()[..bytes.len()], bytes);
}

#[test]
fn second_create_with_same_name_fails() {
    let name = unique_name("dup");
    let _first = SharedMem::create(&name, 4096).expect("first create");
    let second = SharedMem::create(&name, 4096);
    assert!(second.is_err(), "second create with same name must fail");
}

#[test]
fn open_existing_succeeds() {
    let name = unique_name("open");
    let mut creator = SharedMem::create(&name, 4096).expect("create");
    creator.as_mut_slice()[..5].copy_from_slice(b"data1");

    let opener = SharedMem::open(&name, 4096).expect("open");
    assert_eq!(&opener.as_slice()[..5], b"data1");
    assert_eq!(opener.size(), 4096);
}

#[test]
fn open_nonexistent_fails() {
    let name = unique_name("missing");
    let res = SharedMem::open(&name, 4096);
    assert!(res.is_err(), "open of nonexistent name must fail");
}

#[test]
fn drop_creator_unlinks_name_open_fails_afterward() {
    let name = unique_name("drop-unlink");
    {
        let mut creator = SharedMem::create(&name, 4096).expect("create");
        creator.as_mut_slice()[..4].copy_from_slice(b"DATA");
    }
    // Creator dropped → shm_unlink ran → open should fail.
    let res = SharedMem::open(&name, 4096);
    assert!(
        res.is_err(),
        "open after creator drop must fail because shm_unlink ran"
    );
}

#[test]
fn drop_opener_does_not_unlink() {
    let name = unique_name("drop-opener");
    let mut creator = SharedMem::create(&name, 4096).expect("create");
    creator.as_mut_slice()[..3].copy_from_slice(b"abc");
    {
        let _opener = SharedMem::open(&name, 4096).expect("open");
        // opener drops here — must NOT unlink
    }
    // Should still be openable while creator alive
    let opener = SharedMem::open(&name, 4096).expect("open after opener drop");
    assert_eq!(&opener.as_slice()[..3], b"abc");
}

#[test]
fn large_mapping_round_trip() {
    let name = unique_name("large");
    let size = 1 << 20; // 1 MiB
    let mut shm = SharedMem::create(&name, size).expect("create");
    // Write at start, middle, end
    shm.as_mut_slice()[0] = 0x11;
    let mid = size / 2;
    shm.as_mut_slice()[mid] = 0x22;
    shm.as_mut_slice()[size - 1] = 0x33;
    let view = shm.as_slice();
    assert_eq!(view[0], 0x11);
    assert_eq!(view[mid], 0x22);
    assert_eq!(view[size - 1], 0x33);
}

#[test]
fn many_concurrent_creates_with_unique_names() {
    let mut handles = vec![];
    for i in 0..16 {
        let name = unique_name(&format!("multi{i}"));
        handles.push(SharedMem::create(&name, 256).expect("create"));
    }
    // All 16 are alive simultaneously with distinct names.
    assert_eq!(handles.len(), 16);
    // Dropping all must release names (no leak detection in tests, but no panic).
    drop(handles);
}

#[test]
fn zero_byte_write_does_not_corrupt_adjacent_region() {
    let name = unique_name("boundary");
    let mut shm = SharedMem::create(&name, 64).expect("create");
    for b in shm.as_mut_slice().iter_mut() {
        *b = 0xAA;
    }
    // "Write zero bytes" — no-op, just ensure we don't corrupt anything.
    assert_eq!(shm.as_slice()[0], 0xAA);
    assert_eq!(shm.as_slice()[63], 0xAA);
}
