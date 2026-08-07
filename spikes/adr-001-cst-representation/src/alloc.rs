//! Counting global allocator.
//!
//! Memory is the axis MASTER_PLAN §3.1 names, so it has to be measured rather
//! than estimated. RSS would be the easy option and the wrong one: it is
//! polluted by the allocator's own arenas, by the corpus buffers, and by page
//! granularity, and it is not reproducible run to run.
//!
//! Counting every `alloc`/`dealloc` instead gives a number that is **exactly
//! reproducible** — the same input produces the same byte count on every run
//! and every machine. That matters here: it means the memory column of the
//! ADR-001 table is evidence, not an anecdote.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static ALLOCS: AtomicUsize = AtomicUsize::new(0);

pub struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            let live = LIVE.fetch_add(layout.size(), Relaxed) + layout.size();
            PEAK.fetch_max(live, Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let out = unsafe { System.realloc(ptr, layout, new_size) };
        if !out.is_null() {
            ALLOCS.fetch_add(1, Relaxed);
            LIVE.fetch_sub(layout.size(), Relaxed);
            let live = LIVE.fetch_add(new_size, Relaxed) + new_size;
            PEAK.fetch_max(live, Relaxed);
        }
        out
    }
}

/// Live bytes right now.
pub fn live() -> usize {
    LIVE.load(Relaxed)
}

/// Allocation calls so far.
pub fn allocs() -> usize {
    ALLOCS.load(Relaxed)
}

/// Reset the peak watermark to the current live figure.
pub fn reset_peak() {
    PEAK.store(LIVE.load(Relaxed), Relaxed);
}

/// Peak live bytes since the last [`reset_peak`].
pub fn peak() -> usize {
    PEAK.load(Relaxed)
}
