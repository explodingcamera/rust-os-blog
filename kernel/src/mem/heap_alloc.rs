use buddy_system_allocator::LockedHeap;

use crate::println;

#[global_allocator]
static KERNEL_HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::new();
static KERNEL_HEAP: [u8; 0x20000] = [0; 0x20000];

/// Initialize the heap allocator.
pub unsafe fn init_kernel_heap() {
    unsafe {
        let heap_start = KERNEL_HEAP.as_ptr() as usize;
        let heap_size = KERNEL_HEAP.len();
        KERNEL_HEAP_ALLOCATOR.lock().init(heap_start, heap_size);
    }
}
