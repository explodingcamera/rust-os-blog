use core::arch::asm;

use riscv::asm;

use crate::{
    println,
    util::{shutdown, sstc},
};

#[unsafe(export_name = "DefaultHandler")]
fn default_handler() {
    println!("Default handler called\n");
}

#[unsafe(export_name = "SupervisorTimer")]
fn supervisor_timer_handler() {
    println!("SupervisorTimer called");
    // clear the timer interrupt
    sstc::write(usize::MAX);
}

#[unsafe(export_name = "ExceptionHandler")]
fn custom_exception_handler(trap_frame: &riscv_rt::TrapFrame) -> ! {
    println!("Exception handler called");
    println!("Trap frame: {:?}", trap_frame);

    let cause = riscv::register::scause::read();
    panic!("Exception cause: {:?}", cause.cause());
}

#[unsafe(export_name = "UserTimer")]
fn custom_user_timer_handler(trap_frame: &riscv_rt::TrapFrame) {
    println!("User timer handler called");
    println!("Trap frame: {:?}", trap_frame);
}
