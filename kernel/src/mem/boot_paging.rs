//! Identity-map the boot kernel before enabling Sv39.

use super::paging::{Flags, PAGE_SIZE, PageSize, PageTable, PhysAddr, VirtAddr};

const TABLE_COUNT: usize = 16;

#[repr(C, align(4096))]
struct Table([u64; 512]);

// Boot-only table frames for the image and device tree.
static mut TABLES: [Table; TABLE_COUNT] = [const { Table([0; 512]) }; TABLE_COUNT];

unsafe extern "C" {
    static __stext: u8;
    static __etext: u8;
    static __erodata: u8;
    static _heap_guard_start: u8;
    static _heap_guard_end: u8;
    static _stack_guard_start: u8;
    static _stack_start: u8;
}

/// Map the boot image and device tree, then switch this hart to Sv39.
///
/// # Safety
/// Call once on the boot hart, before using other address spaces. The device
/// tree must remain in memory, and other harts must not use this page table.
pub unsafe fn init(device_tree_address: usize) {
    let frames = (&raw mut TABLES).cast::<Table>();
    let root = PhysAddr(frames as usize);
    let mut used = 1;
    let mut table = unsafe { PageTable::new(root) };
    let mut allocate = || {
        if used == TABLE_COUNT {
            return None;
        }
        let frame = PhysAddr(unsafe { frames.add(used) } as usize);
        used += 1;
        Some(frame)
    };

    let start = (&raw const __stext) as usize;
    let text_end = (&raw const __etext) as usize;
    let rodata_end = (&raw const __erodata) as usize;
    let stack_guard = (&raw const _stack_guard_start) as usize;
    let stack_end = (&raw const _stack_start) as usize;
    let heap_guards = [
        (&raw const _heap_guard_start) as usize,
        (&raw const _heap_guard_end) as usize,
    ];
    for address in (start..stack_end).step_by(PAGE_SIZE) {
        if address == stack_guard || heap_guards.contains(&address) {
            continue;
        }
        let mut flags = Flags::READ;
        if address < text_end {
            flags |= Flags::EXECUTE;
        }
        if address + PAGE_SIZE > rodata_end {
            flags |= Flags::WRITE;
        }
        assert!(
            !flags.contains(Flags::WRITE) || !flags.contains(Flags::EXECUTE),
            "boot image has a writable executable page"
        );
        unsafe {
            table.map(
                VirtAddr(address),
                PhysAddr(address),
                PageSize::Size4K,
                flags,
                &mut allocate,
            )
        }
        .expect("failed to map boot kernel");
    }

    let tree = crate::device_tree::DEVICE_TREE
        .get()
        .expect("device tree not initialized");
    let tree_end = device_tree_address
        .checked_add(tree.total_size())
        .expect("device tree too large");
    let mut address = device_tree_address & !(PAGE_SIZE - 1);
    while address < tree_end {
        if table.translate(VirtAddr(address)).is_none() {
            unsafe {
                table.map(
                    VirtAddr(address),
                    PhysAddr(address),
                    PageSize::Size4K,
                    Flags::READ,
                    &mut allocate,
                )
            }
            .expect("failed to map device tree");
        }
        address = address
            .checked_add(PAGE_SIZE)
            .expect("device tree address overflow");
    }

    unsafe { table.activate(0) };
}
