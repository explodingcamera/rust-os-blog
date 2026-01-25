#![no_std]
#![no_main]
#![allow(unused)]

extern crate alloc;
extern crate riscv_rt;

mod panic_handler;
mod trap;

mod dtb;
mod io;
mod ksched;
mod mem;
mod symbols;
mod tasks;
mod timer;
mod util;

use alloc::{boxed::Box, string::String, vec, vec::Vec};
use riscv_rt::{core_interrupt, entry};
use tasks::Fuse;

// the entry point of the kernel, this is only called by hart 0.
// the other harts are busy looping
#[entry]
fn main(a0: usize, a1: usize, a2: usize) -> ! {
    let hart_id = a0;

    println!("== starting pogos kernel on hart {} ==", hart_id);
    println!();

    // initialize the device tree
    dtb::init(a1);

    // Setup everything required for the kernel to run
    unsafe {
        riscv::register::sie::set_stimer();
        riscv::register::sstatus::set_sie();

        // initialize the kernel heap allocator, alloc is now available
        mem::heap_alloc::init_kernel_heap();
        println!(">>> kernel heap initialized");
    }

    println!("kernel initialized, starting kschedule\n");

    ksched::KernelScheduler::new()
        .with_task(ksched::Task::new(tasks::shell_task(), 0))
        .block_on_run();

    util::shutdown()
}
