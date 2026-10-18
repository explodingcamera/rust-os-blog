#![no_std]
#![no_main]
#![allow(unused)]

extern crate alloc;
extern crate riscv_rt;

mod interrupt;

mod device_tree;
mod mem;
mod sbi;
mod scheduler;
mod tasks;

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use riscv_rt::{core_interrupt, entry};
use tasks::Fuse;

pub type Result<T> = core::result::Result<T, &'static str>;

// the entry point of the kernel, this is only called by hart 0.
// the other harts are busy looping
#[entry]
fn main(a0: usize, a1: usize, a2: usize) -> ! {
    let hart_id = a0;

    println!("== starting pogos kernel on hart {} ==", hart_id);
    println!();

    // initialize the device tree
    device_tree::init(a1);

    // Setup everything required for the kernel to run
    unsafe { mem::heap_alloc::init_kernel_heap() };
    println!(">>> kernel heap initialized");

    unsafe { mem::boot_paging::init(a1) };
    println!(">>> kernel paging enabled");

    interrupt::init();
    println!(">>> timer interrupts enabled");

    println!(">>> starting scheduler");
    scheduler::KernelScheduler::new()
        .with_task(scheduler::Task::new(tasks::shell_task(), 0))
        .block_on_run();

    sbi::shutdown()
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{info}");
    sbi::reset(
        sbi_spec::srst::RESET_TYPE_SHUTDOWN,
        sbi_spec::srst::RESET_REASON_SYSTEM_FAILURE,
    )
}
