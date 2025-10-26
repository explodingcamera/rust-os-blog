use riscv::register::{sip, stvec, time};

use crate::{
    println,
    util::{Result, sstc},
};

// get the current time in milliseconds
pub fn get_time() -> Result<u64> {
    let timebase_freq = crate::dtb::DEVICE_TREE
        .get()
        .expect("device tree not initialized")
        .timebase_freq()?;

    Ok(riscv::register::time::read64() / (timebase_freq as u64 / 1000))
}

pub fn set_interrupt(time: usize) -> Result<()> {
    // sbi::timer::set_timer(time).map_err(|_| "set timer failed")
    sstc::write(time::read() + time);

    Ok(())
}
