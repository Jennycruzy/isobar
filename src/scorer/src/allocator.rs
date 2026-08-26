//! Small deterministic bump allocator used by the WASM host ABI.
//!
//! The scorer itself does not allocate. Hosts allocate input buffers through
//! `alloc`, copy UTF-8 bytes into them, call an exported scorer function, and
//! release the buffers through `dealloc`. The arena is reset once all active
//! allocations have been released, which keeps repeated host calls bounded.

use core::cell::UnsafeCell;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const HEAP_SIZE: usize = 2 * 1024 * 1024;
const ALIGNMENT: usize = 8;
const ALIGN_MASK: usize = ALIGNMENT - 1;

#[repr(align(16))]
struct AlignedHeap([u8; HEAP_SIZE]);

struct HeapCell(UnsafeCell<AlignedHeap>);

// The host ABI is synchronous. The wrapper makes the interior-mutability
// contract explicit to the compiler without relying on a hash map or runtime
// allocator.
unsafe impl Sync for HeapCell {}

static HEAP: HeapCell = HeapCell(UnsafeCell::new(AlignedHeap([0; HEAP_SIZE])));
static NEXT: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn align_up(size: usize) -> Option<usize> {
    size.checked_add(ALIGN_MASK)
        .map(|value| value & !ALIGN_MASK)
}

/// Allocate `size` bytes from the module arena.
pub fn alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return ptr::null_mut();
    }

    let Some(aligned) = align_up(size) else {
        return ptr::null_mut();
    };

    let mut current = NEXT.load(Ordering::Relaxed);
    loop {
        let Some(end) = current.checked_add(aligned) else {
            return ptr::null_mut();
        };
        if end > HEAP_SIZE {
            return ptr::null_mut();
        }

        match NEXT.compare_exchange_weak(current, end, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(_) => {
                ACTIVE.fetch_add(1, Ordering::AcqRel);
                // `current < HEAP_SIZE` was checked above.
                return unsafe { (*HEAP.0.get()).0.as_mut_ptr().add(current) };
            }
            Err(observed) => current = observed,
        }
    }
}

/// Release an allocation. Individual blocks are not reused out of order;
/// once the last active allocation is gone, the whole arena is reset.
pub fn dealloc(_ptr: *mut u8, _size: usize) {
    let mut active = ACTIVE.load(Ordering::Acquire);
    loop {
        if active == 0 {
            return;
        }
        match ACTIVE.compare_exchange_weak(active, active - 1, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => {
                if active == 1 {
                    NEXT.store(0, Ordering::Release);
                }
                return;
            }
            Err(observed) => active = observed,
        }
    }
}
