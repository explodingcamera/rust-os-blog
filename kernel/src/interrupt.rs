use core::arch::asm;
use riscv::register::time;
use riscv_rt::{core_interrupt, exception};

use crate::{Result, println, sbi::shutdown};

pub fn init() {
    // Disarm the timer before enabling its interrupt.
    set_timer(usize::MAX);
    unsafe {
        riscv::register::sie::set_stimer();
        riscv::register::sstatus::set_sie();
    }
}

pub fn get_time() -> Result<u64> {
    let ticks_per_ms = crate::device_tree::timebase_freq()? / 1000;
    if ticks_per_ms == 0 {
        return Err("timebase frequency too low");
    }
    Ok(time::read64() / ticks_per_ms)
}

pub fn set_interrupt(delay: usize) -> Result<()> {
    set_timer(time::read().checked_add(delay).ok_or("timer overflow")?);
    Ok(())
}

fn set_timer(deadline: usize) {
    // 0x14d is the stimecmp CSR.
    unsafe { asm!("csrw 0x14d, {}", in(reg) deadline) };
}

#[core_interrupt(riscv::interrupt::Interrupt::SupervisorTimer)]
fn supervisor_timer_handler() {
    println!("SupervisorTimer called");
    // clear the timer interrupt
    set_timer(usize::MAX);
}

#[exception(riscv::interrupt::Exception::UserEnvCall)]
fn user_call(trap_frame: &riscv_rt::TrapFrame) -> ! {
    fatal_exception(trap_frame)
}

#[exception(riscv::interrupt::Exception::IllegalInstruction)]
fn illegal_instruction_handler(trap_frame: &riscv_rt::TrapFrame) -> ! {
    fatal_exception(trap_frame)
}

#[exception(riscv::interrupt::Exception::Breakpoint)]
fn breakpoint_handler(trap_frame: &riscv_rt::TrapFrame) -> ! {
    fatal_exception(trap_frame)
}

fn fatal_exception(trap_frame: &riscv_rt::TrapFrame) -> ! {
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
