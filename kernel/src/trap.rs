use core::arch::asm;

use riscv::asm;
use riscv_rt::{core_interrupt, exception, exceptions};

use crate::{
    println,
    util::{shutdown, sstc},
};

#[core_interrupt(riscv::interrupt::Interrupt::SupervisorTimer)]
fn supervisor_timer_handler() {
    println!("SupervisorTimer called");
    // clear the timer interrupt
    sstc::write(usize::MAX);
}

#[core_interrupt(riscv::interrupt::Interrupt::MachineTimer)]
fn machine_timer_handler() {
    println!("SupervisorTimer called");
    // clear the timer interrupt
    sstc::write(usize::MAX);
}

#[exception(riscv::interrupt::Exception::UserEnvCall)]
fn user_call(trap_frame: &riscv_rt::TrapFrame) -> ! {
    println!("Exception handler called");
    println!("Trap frame: {:?}", trap_frame);

    let cause = riscv::register::scause::read();
    panic!("Exception cause: {:?}", cause.cause());
}

#[exception(riscv::interrupt::Exception::IllegalInstruction)]
fn illegal_instruction_handler(trap_frame: &riscv_rt::TrapFrame) -> ! {
    println!("Exception handler called");
    println!("Trap frame: {:?}", trap_frame);

    let cause = riscv::register::scause::read();
    panic!("Exception cause: {:?}", cause.cause());
}

#[unsafe(export_name = "DefaultHandler")]
unsafe fn fallback_interrupt_handler() {
    println!("interrupt handler called");
    let cause = riscv::register::scause::read();
    println!("cause: {:?}", cause.cause());
    shutdown()
}
