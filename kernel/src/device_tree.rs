use fdt::Fdt;
use spin::Once;

use crate::{Result, println};

pub static DEVICE_TREE: Once<Fdt<'static>> = Once::new();

pub fn init(address: usize) {
    DEVICE_TREE
        .call_once(|| unsafe { Fdt::from_ptr(address as *const u8).expect("invalid device tree") });
    println!(">>> device tree initialized");
}

pub fn timebase_freq() -> Result<u64> {
    let tree = DEVICE_TREE.get().ok_or("device tree not initialized")?;
    tree.find_node("/cpus")
        .and_then(|node| node.property("timebase-frequency"))
        .and_then(|property| property.as_usize())
        .map(|frequency| frequency as u64)
        .ok_or("missing timebase frequency")
}

pub fn print_dtb() {
    let tree = DEVICE_TREE.get().expect("device tree not initialized");
    for node in tree.all_nodes() {
        println!(">>> node: {}", node.name);
        for property in node.properties() {
            println!("    prop: {}", property.name);
        }
    }
}
