use alloc::{ffi::CString, vec, vec::Vec};
use sbi::PhysicalAddress;

pub fn debug_write(t: &[u8]) {
    let num_bytes = t.len();

    unsafe {
        // get the address of the string
        let base_addr = t.as_ptr() as usize;
        let base_addr_lo = base_addr & 0xFFFFFFFF;
        let base_addr_hi = (base_addr >> 32);

        let error: isize;
        let value: usize;

        let _ = sbi::debug_console::write(
            PhysicalAddress::new(base_addr_lo),
            PhysicalAddress::new(base_addr_hi),
            num_bytes,
        );
    }
}

pub fn debug_read(num_bytes: usize) -> Option<Vec<u8>> {
    let mut buffer = vec![0; num_bytes];

    unsafe {
        // get the address of the string
        let base_addr = buffer.as_mut_ptr() as usize;
        let base_addr_lo = base_addr & 0xFFFFFFFF;
        let base_addr_hi = (base_addr >> 32);

        let error: isize;
        let value: usize;

        let _ = sbi::debug_console::read(
            PhysicalAddress::new(base_addr_lo),
            PhysicalAddress::new(base_addr_hi),
            num_bytes,
        );
    }

    Some(buffer)
}
