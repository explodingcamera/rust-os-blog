use alloc::sync::Arc;
use alloc::vec::Vec;
use fdt_rs::base::*;
use fdt_rs::prelude::*;
use spin::once::Once;

use crate::println;
use crate::util::Result;

pub struct DeviceTree(DevTree<'static>);

impl DeviceTree {
    pub fn start(&self) -> usize {
        self.0.buf().as_ptr() as usize
    }

    pub fn end(&self) -> usize {
        self.start() + self.0.buf().len()
    }

    pub fn get_node(&self, name: &str) -> Result<DevTreeNode<'_, '_>> {
        self.0
            .nodes()
            .find(|prop| Ok(prop.name() == Ok(name)))
            .map_err(|_| "node not found")
            .and_then(|node| node.ok_or("node not found"))
    }

    pub fn timebase_freq(&self) -> Result<u32> {
        self.get_node("cpus")?
            .get_prop("timebase-frequency")?
            .u32(0)
            .map_err(|_| "failed to read timebase-frequency")
    }
}

trait GetProp {
    fn get_prop(&self, name: &str) -> Result<DevTreeProp<'_, '_>>;
}

impl GetProp for DevTreeNode<'_, '_> {
    fn get_prop(&self, name: &str) -> Result<DevTreeProp<'_, '_>> {
        let prop = self
            .props()
            .find(|prop| Ok(prop.name() == Ok(name)))
            .map_err(|_| "prop not found")?
            .ok_or("prop not found")?;

        Ok(prop)
    }
}

pub static DEVICE_TREE: Once<DeviceTree> = Once::new();

pub fn print_dtb() {
    let devtree = DEVICE_TREE.get().expect("device tree not initialized").0;
    println!(">>> device tree:");
    let boot_cpuid = devtree.boot_cpuid_phys();
    let root = devtree.root().unwrap().unwrap();
    println!("    boot cpuid: {}", boot_cpuid);

    let mut node_iter = devtree.nodes();
    while let Some(node) = node_iter.next().unwrap() {
        println!(">>> node: {}", node.name().unwrap());

        let mut prop_iter = node.props();
        while let Some(prop) = prop_iter.next().unwrap() {
            println!("    prop: {}", prop.name().unwrap());
        }
    }
}

pub fn read(dtb_addr: usize) -> DeviceTree {
    if dtb_addr == 0 {
        panic!("device tree address is 0, unable to read device tree");
    }

    // Get the actual size of the device tree after reading its header.
    let size = unsafe {
        let header = core::slice::from_raw_parts_mut(dtb_addr as *mut u8, DevTree::MIN_HEADER_SIZE);
        DevTree::read_totalsize(header).expect("failed to read device tree size")
    };

    // Get the device tree buffer
    let buf = unsafe { core::slice::from_raw_parts(dtb_addr as *mut u8, size) };

    // Create the device tree handle
    let devtree = unsafe { DevTree::new(buf).expect("failed to create device tree handle") };

    DeviceTree(devtree)
}

pub fn init(dtb_addr: usize) {
    DEVICE_TREE.call_once(|| read(dtb_addr));
    println!(">>> device tree initialized");
}
