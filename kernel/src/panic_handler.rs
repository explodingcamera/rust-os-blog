use crate::println;
use core::panic::PanicInfo;
use sbi::system_reset::{ResetReason, ResetType};

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("A panic occurred: {info}");

    let _ = sbi::system_reset::system_reset(ResetType::Shutdown, ResetReason::SystemFailure);
    println!("System reset failed");
    loop {}
}
