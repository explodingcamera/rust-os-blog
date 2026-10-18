#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Thread-safe bump allocator. Individual allocations are not freed.
pub struct LinearAllocator {
    head: AtomicUsize,
    start: *mut u8, // pointer to the start of the heap
    size: usize,    // size of the heap in bytes
}

impl LinearAllocator {
    pub const fn empty() -> Self {
        Self {
            head: AtomicUsize::new(0),
            start: ptr::null_mut(),
            size: 0,
        }
    }

    /// # Safety
    /// The region must be writable, unused, and remain available for the
    /// allocator's lifetime. Initialize before allocating, and only once.
    pub unsafe fn init(&mut self, start: usize, size: usize) {
        assert!(start != 0 && size != 0);
        start.checked_add(size).expect("heap address overflow");
        self.start = start as *mut u8;
        self.size = size;
    }
}

unsafe impl Sync for LinearAllocator {}

unsafe impl GlobalAlloc for LinearAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if self.start.is_null() || layout.size() == 0 {
            return ptr::null_mut();
        }

        // the byte multiple that our allocated memory must start at
        let align = layout.align();

        // The size is the number of bytes we need to allocate
        let size = layout.size();

        let base = self.start.addr();

        loop {
            let head = self.head.load(Ordering::Relaxed);

            // Align the pointer, since the heap start may be unaligned.
            let Some(mut address) = base.checked_add(head) else {
                return ptr::null_mut();
            };
            if !address.is_multiple_of(align) {
                let Some(aligned) = address.checked_add(align - address % align) else {
                    return ptr::null_mut();
                };
                address = aligned;
            }

            // Move the head forward by the size of the allocation
            let Some(new_head) = address.checked_add(size) else {
                return ptr::null_mut();
            };

            // are we out of memory?
            if new_head - base > self.size {
                return ptr::null_mut();
            }

            if self
                .head
                .compare_exchange(head, new_head - base, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                return unsafe { self.start.add(address - base) };
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // no-op
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use std::vec::Vec;

    #[test]
    fn aligns_and_stops_at_heap_end() {
        let mut heap = [0_u8; 128];
        let mut allocator = LinearAllocator::empty();
        let layout = Layout::from_size_align(8, 32).unwrap();
        assert!(unsafe { allocator.alloc(layout) }.is_null());
        unsafe { allocator.init(heap.as_mut_ptr() as usize + 1, 64) };
        let pointer = unsafe { allocator.alloc(layout) };
        assert!(!pointer.is_null());
        assert!(pointer.addr().is_multiple_of(32));
        assert!(unsafe { allocator.alloc(Layout::from_size_align(64, 8).unwrap()) }.is_null());
    }

    #[test]
    fn concurrent_allocations_do_not_overlap() {
        let mut heap = [0_u8; 4096];
        let mut allocator = LinearAllocator::empty();
        unsafe { allocator.init(heap.as_mut_ptr() as usize, heap.len()) };
        let layout = Layout::from_size_align(8, 8).unwrap();
        let mut addresses = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..4)
                .map(|_| {
                    let allocator = &allocator;
                    scope.spawn(move || {
                        (0..128)
                            .map(|_| unsafe { allocator.alloc(layout) }.addr())
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            threads
                .into_iter()
                .flat_map(|thread| thread.join().unwrap())
                .collect::<Vec<_>>()
        });
        addresses.sort_unstable();
        assert!(addresses[0] != 0);
        assert!(addresses.windows(2).all(|pair| pair[1] - pair[0] == 8));
    }
}
