#![allow(non_snake_case, dead_code)]

unsafe extern "C" {
    static _stext: u8;

    static _stack_start: u8;
    static _max_hart_id: u8;
    static _hart_stack_size: u8;
    static _heap_size: u8;

    static pog_memory_start: u8;
    static pog_memory_end: u8;
    static pog_etext: u8;
    static pog_srodata: u8;
    static pog_erodata: u8;

    static _sdata: u8;
    static _edata: u8;
    static _sbss: u8;
    static _ebss: u8;
    static _sheap: u8;
    static _eheap: u8;

    // note that the stack grows downwards
    static _sstack: u8;
    static _estack: u8;

    static _end: u8;
}

pub fn MEMORY_START() -> usize {
    unsafe { &pog_memory_start as *const _ as usize }
}

pub fn MEMORY_END() -> usize {
    unsafe { &pog_memory_end as *const _ as usize }
}

pub fn TEXT_START() -> usize {
    unsafe { &_stext as *const _ as usize }
}

pub fn TEXT_END() -> usize {
    unsafe { &pog_etext as *const _ as usize }
}

pub fn RODATA_START() -> usize {
    unsafe { &pog_srodata as *const _ as usize }
}

pub fn RODATA_END() -> usize {
    unsafe { &pog_erodata as *const _ as usize }
}

pub fn DATA_START() -> usize {
    unsafe { &_sdata as *const _ as usize }
}

pub fn DATA_END() -> usize {
    unsafe { &_edata as *const _ as usize }
}

pub fn BSS_START() -> usize {
    unsafe { &_sbss as *const _ as usize }
}

pub fn BSS_END() -> usize {
    unsafe { &_ebss as *const _ as usize }
}

pub fn STACK_START() -> usize {
    unsafe { &_sstack as *const _ as usize }
}

pub fn STACK_END() -> usize {
    unsafe { &_estack as *const _ as usize }
}

pub fn KERNEL_END() -> usize {
    STACK_START()
}
