//! SBI calls used by the kernel. Buffer addresses are physical while paging is off.

use core::arch::asm;
use core::fmt;
use sbi_spec::{binary::SbiRet, dbcn, srst};

fn call(extension: usize, function: usize, arg0: usize, arg1: usize, arg2: usize) -> SbiRet {
    let mut error = arg0;
    let mut value = arg1;
    // SBI arguments use a0-a2, a6-a7, and results use a0-a1.
    unsafe {
        asm!(
            "ecall",
            inlateout("a0") error,
            inlateout("a1") value,
            in("a2") arg2,
            in("a6") function,
            in("a7") extension,
        );
    }
    SbiRet { error, value }
}

pub fn write(mut bytes: &[u8]) {
    while !bytes.is_empty() {
        let result = call(
            dbcn::EID_DBCN,
            dbcn::CONSOLE_WRITE,
            bytes.len(),
            bytes.as_ptr() as usize,
            0,
        );
        if result.is_err() || result.value == 0 {
            break;
        }
        bytes = &bytes[result.value.min(bytes.len())..];
    }
}

pub fn read_byte() -> Option<u8> {
    let mut byte = [0];
    let result = call(
        dbcn::EID_DBCN,
        dbcn::CONSOLE_READ,
        1,
        byte.as_mut_ptr() as usize,
        0,
    );
    (result.is_ok() && result.value == 1).then_some(byte[0])
}

pub fn reset(reset_type: u32, reason: u32) -> ! {
    let _ = call(
        srst::EID_SRST,
        srst::SYSTEM_RESET,
        reset_type as usize,
        reason as usize,
        0,
    );
    loop {
        riscv::asm::wfi();
    }
}

pub fn shutdown() -> ! {
    reset(srst::RESET_TYPE_SHUTDOWN, srst::RESET_REASON_NO_REASON)
}

pub fn reboot() -> ! {
    reset(srst::RESET_TYPE_COLD_REBOOT, srst::RESET_REASON_NO_REASON)
}

#[macro_export]
macro_rules! print {
    ($fmt:literal$(, $($arg: tt)+)?) => {
        $crate::sbi::SBIConsole::print_args(core::format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt:literal$(, $($arg: tt)+)?) => {{
        $crate::print!($fmt $(,$($arg)+)?);
        $crate::sbi::SBIConsole::print("\n");
    }};
    () => {
        $crate::sbi::SBIConsole::print("\n");
    }
}

pub struct SBIConsole;

impl SBIConsole {
    pub fn read_byte() -> Option<u8> {
        read_byte()
    }

    pub fn print(s: &str) {
        write(s.as_bytes());
    }

    pub fn print_args(args: fmt::Arguments<'_>) {
        use fmt::Write;
        SBIConsole.write_fmt(args).unwrap();
    }
}

impl fmt::Write for SBIConsole {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Self::print(s);
        Ok(())
    }
}
