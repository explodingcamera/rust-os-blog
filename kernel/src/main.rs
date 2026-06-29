#![no_std]
#![no_main]

// extern crate alloc;
extern crate riscv_rt;

use riscv_rt::entry;
mod panic_handler;
mod utils;

#[entry]
fn main(a0: usize) -> ! {
    println!("Hello world from hart {}!", a0);
    utils::shutdown();
}
