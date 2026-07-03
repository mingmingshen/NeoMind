//! Cross-process shared memory primitive + SPSC ring buffer for the NeoMind
//! PCM fast path.
//!
//! Folded into the SDK in Stage 2 / B.0 from the standalone
//! `neomind-shared-memory` crate. Gated behind the `shm-ring` feature flag
//! because it requires POSIX `shm_open` / `mmap` (Unix only) and the `libc`
//! crate, which the SDK does not depend on by default.
//!
//! See `docs/neomind-pcm-fast-path-design.zh.md` §3-5 for the original
//! design. On macOS / Linux this uses POSIX `shm_open` + `mmap`. The creator
//! owns the name (calls `shm_unlink` on drop); openers only mmap and unmap.
//!
//! Single-process semantics are tested in `tests/shm_ring.rs`. Cross-process
//! behavior is kernel-provided (any process holding the name can mmap the
//! same pages) and is exercised by integration tests in the runner crate.

#![cfg(all(unix, feature = "shm-ring"))]

use std::ffi::CString;
use std::io;
use std::os::unix::io::RawFd;
use std::ptr;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::Duration;

// ============================================================================
// SharedMem — POSIX shm_open + mmap wrapper
// ============================================================================

/// Cross-process shared memory segment.
///
/// `owned = true` means this handle was created via [`SharedMem::create`] and
/// is responsible for calling `shm_unlink` on drop. Openers have `owned =
/// false` and only unmap on drop.
pub struct SharedMem {
    name: CString,
    ptr: *mut u8,
    size: usize,
    fd: RawFd,
    owned: bool,
}

unsafe impl Send for SharedMem {}
unsafe impl Sync for SharedMem {}

impl SharedMem {
    /// Create a new shared memory segment of `size` bytes. Fails if a segment
    /// with the same name already exists (POSIX `EEXIST`).
    pub fn create(name: &str, size: usize) -> io::Result<Self> {
        let cname = name_to_c(name)?;
        // O_CREAT | O_EXCL: fail if exists. 0600: owner read/write.
        let fd = unsafe {
            libc::shm_open(
                cname.as_ptr(),
                libc::O_CREAT | libc::O_EXCL | libc::O_RDWR,
                0o600,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // Resize to requested size. ftruncate zero-fills.
        let r = unsafe { libc::ftruncate(fd, size as libc::off_t) };
        if r < 0 {
            let err = io::Error::last_os_error();
            unsafe {
                libc::close(fd);
                libc::shm_unlink(cname.as_ptr());
            }
            return Err(err);
        }
        let ptr = map(fd, size)?;
        Ok(Self {
            name: cname,
            ptr,
            size,
            fd,
            owned: true,
        })
    }

    /// Open an existing shared memory segment by name. Fails if the segment
    /// does not exist (`ENOENT`). The opener does NOT call `shm_unlink` on
    /// drop — only the creator unlinks.
    pub fn open(name: &str, size: usize) -> io::Result<Self> {
        let cname = name_to_c(name)?;
        let fd = unsafe { libc::shm_open(cname.as_ptr(), libc::O_RDWR, 0o600) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let ptr = map(fd, size)?;
        Ok(Self {
            name: cname,
            ptr,
            size,
            fd,
            owned: false,
        })
    }

    pub fn name(&self) -> String {
        // CString → &str is safe (we constructed from valid UTF-8).
        self.name.to_string_lossy().into_owned()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

impl Drop for SharedMem {
    fn drop(&mut self) {
        unsafe {
            // Unmap regardless of ownership.
            if !self.ptr.is_null() && self.size > 0 {
                libc::munmap(self.ptr as *mut libc::c_void, self.size);
            }
            libc::close(self.fd);
            // Only the creator unlinks the name.
            if self.owned {
                libc::shm_unlink(self.name.as_ptr());
            }
        }
    }
}

fn map(fd: RawFd, size: usize) -> io::Result<*mut u8> {
    if size == 0 {
        return Ok(ptr::null_mut());
    }
    let ptr = unsafe {
        libc::mmap(
            ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd,
            0,
        )
    };
    if ptr == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    Ok(ptr as *mut u8)
}

fn name_to_c(name: &str) -> io::Result<CString> {
    // POSIX requires the name to start with '/' and contain no embedded '/'.
    if !name.starts_with('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "shared memory name must start with '/'",
        ));
    }
    if name.len() > 1 && name[1..].contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "shared memory name must not contain '/' after the leading slash",
        ));
    }
    CString::new(name).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "shared memory name contains NUL byte",
        )
    })
}

// ============================================================================
// SPSC ring buffer over a SharedMem segment
// ============================================================================
//
// Layout (see `docs/neomind-pcm-fast-path-design.zh.md` §5):
// ```text
// ┌──────────RingHeader (64 B)──────────┐
// │ magic | version | frame_size_max    │
// │ frame_count_max, write_pos (u64)    │
// │ read_pos (u64), dropped_count (u64) │
// │ flags (u32), padding (28 B)         │
// ├──────FrameSlot[0..N] (16 B each)────┤
// │ len (u32) | ts_ns (u64) | flags | p │
// ├──────Data pool (N * frame_size)─────┤
// ```
//
// Single writer, single reader. Writer advances `write_pos`; reader advances
// `read_pos`. Both must agree on `frame_size_max` and `frame_count_max`
// (carried in the header so the reader can self-describe on `open`).

/// Magic constant stored in `RingHeader::magic`. "NOMD" little-endian.
pub const RING_MAGIC: u32 = 0x4E4F4D44;

/// `RingHeader::version` — bump if layout changes.
pub const RING_VERSION: u32 = 1;

/// `RingHeader::flags` bit 0 — set by writer to signal end of stream.
pub const FLAG_CLOSED: u32 = 0x1;

/// `RingHeader` layout, exactly one cache line (64 B).
#[repr(C)]
pub struct RingHeader {
    pub magic: u32,
    pub version: u32,
    pub frame_size_max: u32,
    pub frame_count_max: u32,
    pub write_pos: AtomicU64,
    pub read_pos: AtomicU64,
    pub dropped_count: AtomicU64,
    pub flags: AtomicU32,
    // Explicit pad to bring total to 64 B (44 used → 20 pad).
    _padding: [u8; 20],
}

const _: () = {
    // Compile-time layout check.
    assert!(std::mem::size_of::<RingHeader>() == 64);
};

/// Per-slot metadata, prepended to each data frame.
#[repr(C)]
pub struct FrameSlot {
    /// Actual data length in bytes (0 = empty/reserved).
    pub len: AtomicU32,
    /// Write sequence counter for SPSC race protection (Disruptor-style).
    /// Writer stores `lap*2+1` (odd = writing) before the copy, then
    /// `lap*2+2` (even = ready) after the copy. Reader checks the expected
    /// even value before AND after its copy to detect concurrent overwrites.
    pub seq: AtomicU32,
    /// Write timestamp (nanoseconds, arbitrary epoch).
    pub ts_ns: AtomicU64,
}

const _: () = {
    assert!(std::mem::size_of::<FrameSlot>() == 16);
};

/// What to do when the ring is full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropPolicy {
    /// Overwrite the oldest unread frame (live-audio default).
    DropOldest,
    /// Drop the incoming frame (best-effort delivery default).
    DropNewest,
}

/// Writer-side outcome of a `write` call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteResult {
    /// Wrote the frame at the given slot index.
    Written(u64),
    /// Ring was full and policy is `DropNewest` — frame was discarded.
    Dropped,
    /// `data.len()` exceeds the configured `frame_size_max`.
    InvalidSize,
}

/// Reader-side view of a frame.
#[derive(Debug, Clone)]
pub struct ReadFrame<'a> {
    /// The actual payload bytes (slice of the caller-provided buffer).
    pub buf: &'a [u8],
    /// Timestamp captured at write time.
    pub ts_ns: u64,
    /// Reserved (always 0; the slot's seq field is internal).
    pub flags: u32,
}

/// Owner of the backing segment. Drops unlinks the segment.
pub struct RingHandle {
    shm: SharedMem,
    frame_size_max: u32,
    frame_count_max: u32,
    policy: DropPolicy,
}

impl RingHandle {
    /// Create a new ring buffer of the given dimensions. The segment is owned
    /// (unlinked on drop). Openers on other processes must call
    /// [`open_sized`](Self::open_sized) with the same name and dimensions
    /// (they read `frame_size_max` / `frame_count_max` from the StreamOpened
    /// response, not from the segment itself).
    pub fn create(
        name: &str,
        frame_size_max: u32,
        frame_count_max: u32,
        policy: DropPolicy,
    ) -> io::Result<Self> {
        let total = layout_total(frame_size_max, frame_count_max);
        let shm = SharedMem::create(name, total)?;
        // Initialize header.
        let header = header_ptr(&shm);
        unsafe {
            (*header).magic = RING_MAGIC;
            (*header).version = RING_VERSION;
            (*header).frame_size_max = frame_size_max;
            (*header).frame_count_max = frame_count_max;
            (*header).write_pos.store(0, Ordering::Release);
            (*header).read_pos.store(0, Ordering::Release);
            (*header).dropped_count.store(0, Ordering::Release);
            (*header).flags.store(0, Ordering::Release);
        }
        Ok(Self {
            shm,
            frame_size_max,
            frame_count_max,
            policy,
        })
    }

    /// Open with explicit dimensions (used by the runner after it receives
    /// `frame_size_max` / `frame_count_max` from the StreamOpened response).
    pub fn open_sized(
        name: &str,
        frame_size_max: u32,
        frame_count_max: u32,
    ) -> io::Result<Self> {
        let total = layout_total(frame_size_max, frame_count_max);
        let shm = SharedMem::open(name, total)?;
        // Validate magic.
        let header = header_ptr(&shm);
        let magic = unsafe { (*header).magic };
        if magic != RING_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("bad magic: 0x{magic:08x}"),
            ));
        }
        Ok(Self {
            shm,
            frame_size_max,
            frame_count_max,
            policy: DropPolicy::DropOldest,
        })
    }

    /// Borrow as a writer. The handle outlives the writer. Uses the policy
    /// configured at `create` time.
    pub fn writer(&self) -> PcmRingWriter<'_> {
        let header = header_ptr(&self.shm);
        let slots_base = unsafe { (header as *const u8).add(std::mem::size_of::<RingHeader>()) }
            as *mut FrameSlot;
        let data_base = unsafe {
            (slots_base as *const u8).add(std::mem::size_of::<FrameSlot>() * self.frame_count_max as usize)
        } as *mut u8;
        PcmRingWriter {
            header,
            slots: slots_base,
            data: data_base,
            frame_size_max: self.frame_size_max,
            frame_count_max: self.frame_count_max,
            policy: self.policy,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Borrow as a reader.
    pub fn reader(&self) -> PcmRingReader<'_> {
        let header = header_ptr(&self.shm);
        let slots_base = unsafe { (header as *const u8).add(std::mem::size_of::<RingHeader>()) }
            as *mut FrameSlot;
        let data_base = unsafe {
            (slots_base as *const u8).add(std::mem::size_of::<FrameSlot>() * self.frame_count_max as usize)
        } as *mut u8;
        PcmRingReader {
            header,
            slots: slots_base,
            data: data_base,
            frame_size_max: self.frame_size_max,
            frame_count_max: self.frame_count_max,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Override the default drop policy used by `writer()`. The reader also
    /// gets the policy by reading the writer's borrow — but typical usage is
    /// to construct a fresh writer with the desired policy each time.
    pub fn writer_with_policy(&self, policy: DropPolicy) -> PcmRingWriter<'_> {
        let mut w = self.writer();
        w.policy = policy;
        w
    }
}

/// Writer side of the ring. Lifetime ties it to the [`RingHandle`] that
/// vends it (the handle owns the backing segment).
pub struct PcmRingWriter<'a> {
    header: *mut RingHeader,
    slots: *mut FrameSlot,
    data: *mut u8,
    frame_size_max: u32,
    frame_count_max: u32,
    policy: DropPolicy,
    _phantom: std::marker::PhantomData<&'a RingHandle>,
}

impl<'a> PcmRingWriter<'a> {
    /// Push one frame. See [`WriteResult`] for outcomes.
    ///
    /// **SPSC contract**: the writer only mutates `write_pos`; the reader only
    /// mutates `read_pos`. Under `DropPolicy::DropOldest` the writer does NOT
    /// touch `read_pos` (which would race the reader's own `read_pos` advance
    /// and silently skip frames). Instead the writer writes at
    /// `slot = write_pos % capacity` (which equals `read_pos % capacity` when
    /// the ring is full — overwriting the oldest unread frame in place) and
    /// advances `write_pos` by exactly 1, same as a non-overflow write. The
    /// reader detects `gap > capacity` on its next call and fast-forwards its
    /// own `read_pos` to `write_pos - capacity`, skipping the overwritten
    /// frames.
    pub fn write(&self, data: &[u8], ts_ns: u64) -> WriteResult {
        if data.len() as u32 > self.frame_size_max {
            return WriteResult::InvalidSize;
        }
        let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
        let read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };
        let capacity = self.frame_count_max as u64;

        if write_pos - read_pos >= capacity {
            match self.policy {
                DropPolicy::DropOldest => {
                    // Track the drop; the reader will fast-forward past the
                    // overwritten slot on its next read. We do NOT mutate
                    // read_pos here — that would race the reader.
                    unsafe {
                        (*self.header)
                            .dropped_count
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    // Fall through: write at `write_pos % capacity`, which is
                    // the slot the reader is about to read. The len=0
                    // reservation + reader fast-forward makes this safe.
                }
                DropPolicy::DropNewest => {
                    unsafe {
                        (*self.header)
                            .dropped_count
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    return WriteResult::Dropped;
                }
            }
        }

        // Reserve the slot. SPSC contract: no other writer exists.
        let slot_idx = (write_pos % capacity) as usize;
        let slot = unsafe { self.slots.add(slot_idx) };
        // Disruptor-style sequence protocol: store an odd seq (writing) before
        // the copy, then the matching even seq (done) after. The reader checks
        // the expected even value before AND after its copy — if the value
        // changes, the writer overwrote the slot mid-read and the reader
        // retries. This closes the DropOldest race where the reader loads a
        // stale len just before the writer overwrites the slot.
        let lap = write_pos / capacity;
        let seq_writing = (lap * 2 + 1) as u32;
        let seq_done = (lap * 2 + 2) as u32;
        unsafe {
            (*slot).seq.store(seq_writing, Ordering::Release);
            (*slot).len.store(0, Ordering::Release);
        }
        let data_dst = unsafe { self.data.add(slot_idx * self.frame_size_max as usize) };
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), data_dst, data.len());
            (*slot).ts_ns.store(ts_ns, Ordering::Release);
            (*slot).len.store(data.len() as u32, Ordering::Release);
            (*slot).seq.store(seq_done, Ordering::Release);
        }
        // Publish by advancing write_pos by exactly 1.
        unsafe {
            (*self.header)
                .write_pos
                .store(write_pos + 1, Ordering::Release)
        };
        WriteResult::Written(write_pos)
    }

    /// Signal end of stream. Reader will drain remaining frames then observe None.
    pub fn close(&self) {
        unsafe { (*self.header).flags.fetch_or(FLAG_CLOSED, Ordering::Release) };
    }

    /// Total frames dropped due to ring-full conditions.
    pub fn dropped_count(&self) -> u64 {
        unsafe { (*self.header).dropped_count.load(Ordering::Relaxed) }
    }
}

/// Reader side of the ring.
pub struct PcmRingReader<'a> {
    header: *mut RingHeader,
    slots: *mut FrameSlot,
    data: *mut u8,
    frame_size_max: u32,
    frame_count_max: u32,
    _phantom: std::marker::PhantomData<&'a RingHandle>,
}

impl<'a> PcmRingReader<'a> {
    /// Read the next available frame into `out` (must be at least
    /// `frame_size_max` bytes). Returns `None` only when the ring is closed
    /// AND drained.
    ///
    /// **Blocks (busy-spin) until data is available.** This consumes 100% CPU
    /// on one core while spinning. For soft-realtime pipelines that need to
    /// yield, use [`try_read_for`](Self::try_read_for) with a reasonable
    /// deadline, or call this method from a dedicated thread.
    pub fn read<'b>(&self, out: &'b mut [u8]) -> Option<ReadFrame<'b>> {
        loop {
            let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
            let mut read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };
            let capacity = self.frame_count_max as u64;

            // DropOldest catch-up: writer may have let gap exceed capacity.
            if write_pos > read_pos && write_pos - read_pos > capacity {
                let new_read_pos = write_pos - capacity;
                unsafe {
                    (*self.header)
                        .read_pos
                        .store(new_read_pos, Ordering::Release)
                };
                read_pos = new_read_pos;
            }

            if read_pos >= write_pos {
                let flags = unsafe { (*self.header).flags.load(Ordering::Acquire) };
                if flags & FLAG_CLOSED != 0 {
                    return None;
                }
                std::hint::spin_loop();
                continue;
            }
            let slot_idx = (read_pos % capacity) as usize;
            let slot = unsafe { self.slots.add(slot_idx) };

            let lap = read_pos / capacity;
            let expected_seq = (lap * 2 + 2) as u32;
            let seq_before = unsafe { (*slot).seq.load(Ordering::Acquire) };
            if seq_before != expected_seq {
                std::hint::spin_loop();
                continue;
            }

            let len = unsafe { (*slot).len.load(Ordering::Acquire) };
            if len == 0 {
                std::hint::spin_loop();
                continue;
            }
            let data_src = unsafe { self.data.add(slot_idx * self.frame_size_max as usize) };
            unsafe {
                std::ptr::copy_nonoverlapping(data_src, out.as_mut_ptr(), len as usize);
            }
            let seq_after = unsafe { (*slot).seq.load(Ordering::Acquire) };
            if seq_after != expected_seq {
                std::hint::spin_loop();
                continue;
            }
            let ts_ns = unsafe { (*slot).ts_ns.load(Ordering::Relaxed) };
            unsafe { (*self.header).read_pos.fetch_add(1, Ordering::SeqCst) };
            return Some(ReadFrame {
                buf: &out[..len as usize],
                ts_ns,
                flags: 0,
            });
        }
    }

    /// Like `read` but bounded by a timeout. Returns None if no frame arrived
    /// in time (ring may still be open).
    pub fn try_read_for<'b>(
        &self,
        deadline: Duration,
        out: &'b mut [u8],
    ) -> Option<ReadFrame<'b>> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() >= deadline {
                return None;
            }
            let write_pos = unsafe { (*self.header).write_pos.load(Ordering::Acquire) };
            let mut read_pos = unsafe { (*self.header).read_pos.load(Ordering::Acquire) };
            let capacity = self.frame_count_max as u64;

            if write_pos > read_pos && write_pos - read_pos > capacity {
                let new_read_pos = write_pos - capacity;
                unsafe {
                    (*self.header)
                        .read_pos
                        .store(new_read_pos, Ordering::Release)
                };
                read_pos = new_read_pos;
            }

            if read_pos >= write_pos {
                let flags = unsafe { (*self.header).flags.load(Ordering::Acquire) };
                if flags & FLAG_CLOSED != 0 {
                    return None;
                }
                std::hint::spin_loop();
                continue;
            }

            let slot_idx = (read_pos % capacity) as usize;
            let slot = unsafe { self.slots.add(slot_idx) };
            let lap = read_pos / capacity;
            let expected_seq = (lap * 2 + 2) as u32;
            let seq_before = unsafe { (*slot).seq.load(Ordering::Acquire) };
            if seq_before != expected_seq {
                std::hint::spin_loop();
                continue;
            }
            let len = unsafe { (*slot).len.load(Ordering::Acquire) };
            if len == 0 {
                std::hint::spin_loop();
                continue;
            }
            let data_src = unsafe { self.data.add(slot_idx * self.frame_size_max as usize) };
            unsafe {
                std::ptr::copy_nonoverlapping(data_src, out.as_mut_ptr(), len as usize);
            }
            let seq_after = unsafe { (*slot).seq.load(Ordering::Acquire) };
            if seq_after != expected_seq {
                std::hint::spin_loop();
                continue;
            }
            let ts_ns = unsafe { (*slot).ts_ns.load(Ordering::Relaxed) };
            unsafe { (*self.header).read_pos.fetch_add(1, Ordering::SeqCst) };
            return Some(ReadFrame {
                buf: &out[..len as usize],
                ts_ns,
                flags: 0,
            });
        }
    }
}

fn layout_total(frame_size_max: u32, frame_count_max: u32) -> usize {
    std::mem::size_of::<RingHeader>()
        + std::mem::size_of::<FrameSlot>() * frame_count_max as usize
        + frame_size_max as usize * frame_count_max as usize
}

fn header_ptr(shm: &SharedMem) -> *mut RingHeader {
    shm.as_slice().as_ptr() as *mut RingHeader
}

// SAFETY: writer/reader access cross-process shared memory via raw pointers.
unsafe impl<'a> Send for PcmRingWriter<'a> {}
unsafe impl<'a> Sync for PcmRingWriter<'a> {}
unsafe impl<'a> Send for PcmRingReader<'a> {}
unsafe impl<'a> Sync for PcmRingReader<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_without_leading_slash_rejected() {
        let r = name_to_c("no-slash");
        assert!(r.is_err());
    }

    #[test]
    fn name_with_embedded_slash_rejected() {
        let r = name_to_c("/foo/bar");
        assert!(r.is_err());
    }

    #[test]
    fn name_with_single_slash_ok() {
        let r = name_to_c("/only");
        assert!(r.is_ok());
    }

    fn nm(label: &str) -> String {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        let pid = std::process::id();
        let n = N.fetch_add(1, Ordering::SeqCst);
        format!("/nm-ring-unit-{label}-{pid}-{n}")
    }

    #[test]
    fn header_layout_compile_time_check_passes() {
        assert_eq!(std::mem::size_of::<RingHeader>(), 64);
        assert_eq!(std::mem::size_of::<FrameSlot>(), 16);
    }

    #[test]
    fn open_sized_with_bad_magic_fails() {
        let name = nm("badmagic");
        let _unused =
            SharedMem::create(&name, layout_total(8, 4)).expect("create raw segment");
        let r = RingHandle::open_sized(&name, 8, 4);
        assert!(r.is_err());
        drop(_unused);
    }
}
