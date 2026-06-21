use crate::address::PhysPageNum;
use alloc::vec::Vec;
use core::{
    fmt::{self, Debug, Formatter},
    mem,
};

mod stack;
pub use stack::StackFrameAllocator;

#[derive(Clone)]
pub struct FrameTracker {
    pub ppn: PhysPageNum,
}

impl FrameTracker {
    pub fn new(ppn: PhysPageNum) -> Self {
        // page cleaning
        let bytes_array = ppn.get_bytes_array();
        for i in bytes_array {
            *i = 0;
        }

        Self { ppn }
    }
}

impl Debug for FrameTracker {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        // this is a hack to prevent FrameTracker from being dropped
        // without calling frame_dealloc, which would cause a memory leak.
        // We don't have access to the allocator here, so we can't
        // deallocate the page.
        // idea from: https://faultlore.com/blah/linear-rust/

        panic!("FrameTracker should not be dropped, use frame_dealloc instead");
    }
}

pub type FrameAllocFn = fn() -> Option<FrameTracker>;

pub trait FrameAllocator {
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn alloc_multiple(&mut self, pages: usize) -> Option<Vec<PhysPageNum>> {
        let mut v = Vec::new();
        for _ in 0..pages {
            {
                let ppn = self.alloc()?;
                v.push(ppn);
            }
        }
        Some(v)
    }

    fn dealloc(&mut self, ppn: PhysPageNum);

    fn frame_alloc(&mut self) -> Option<FrameTracker> {
        self.alloc().map(FrameTracker::new)
    }

    fn frame_alloc_more(&mut self, num: usize) -> Option<Vec<FrameTracker>> {
        self.alloc_multiple(num)
            .map(|x| x.iter().map(|&t| FrameTracker::new(t)).collect())
    }

    fn frame_dealloc(&mut self, frame: FrameTracker) {
        self.dealloc(frame.ppn);
        mem::forget(frame);
    }

    fn frame_dealloc_ppm(&mut self, ppn: PhysPageNum) {
        self.dealloc(ppn);
    }
}
