use buddy_system_allocator::LockedHeap;

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::new();
unsafe extern "C" {
    static _heap_start: u8;
    static _heap_guard_end: u8;
}

/// Initialize the heap allocator.
///
/// # Safety
/// Call only once, before any heap allocation.
pub unsafe fn init_kernel_heap() {
    unsafe {
        let start = (&raw const _heap_start) as usize;
        let end = (&raw const _heap_guard_end) as usize;
        KERNEL_HEAP_ALLOCATOR.lock().init(start, end - start);
    }
}
