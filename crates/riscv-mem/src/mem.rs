use crate::PAGE_SIZE;
use crate::address::PhysPageNum;
use crate::address::StepByOne;
use crate::address::VPNRange;
use crate::address::VirtAddr;
use crate::address::VirtPageNum;
use crate::frame_allocator::FrameAllocFn;
use crate::frame_allocator::FrameTracker;
use crate::page::PTEFlags;
use crate::page::PageTable;
use crate::page::PageTableEntry;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use bitflags::bitflags;
use riscv::register::satp;

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
    allocate_frame: FrameAllocFn,
}

impl MemorySet {
    pub fn new_bare(allocate_frame: FrameAllocFn) -> Self {
        Self {
            page_table: PageTable::new(allocate_frame),
            areas: Vec::new(),
            allocate_frame,
        }
    }

    pub fn root_ppn(&self) -> PhysPageNum {
        self.page_table.root_ppn()
    }

    pub fn root_token(&self) -> usize {
        self.page_table.root_token()
    }

    /// Assume that no conflicts.
    pub fn insert_framed_area(
        &mut self,
        start_va: VirtAddr,
        end_va: VirtAddr,
        permission: MapPermission,
    ) {
        self.push(
            MapArea::new(
                start_va,
                end_va,
                MapType::Framed,
                permission,
                self.allocate_frame,
            ),
            None,
        );
    }
    pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
        if let Some((idx, area)) = self
            .areas
            .iter_mut()
            .enumerate()
            .find(|(_, area)| area.vpn_range.get_start() == start_vpn)
        {
            area.unmap(&mut self.page_table);
            self.areas.remove(idx);
        }
    }
    /// Add a new MapArea into this MemorySet.
    /// Assuming that there are no conflicts in the virtual address
    /// space.
    pub fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&self.page_table, data);
        }
        self.areas.push(map_area);
    }
    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        //     self.page_table.map(
        //         VirtAddr::from(TRAMPOLINE).into(),
        //         PhysAddr::from(strampoline as usize).into(),
        //         PTEFlags::R | PTEFlags::X,
        //     );
    }

    // /// Include sections in elf and trampoline,
    // /// also returns user_sp_base and entry point.
    // pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
    //     let mut memory_set = Self::new_bare();
    //     // map trampoline
    //     memory_set.map_trampoline();
    //     // map program headers of elf, with U flag
    //     let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
    //     let elf_header = elf.header;
    //     let magic = elf_header.pt1.magic;
    //     assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    //     let ph_count = elf_header.pt2.ph_count();
    //     let mut max_end_vpn = VirtPageNum(0);
    //     for i in 0..ph_count {
    //         let ph = elf.program_header(i).unwrap();
    //         if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
    //             let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
    //             let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
    //             let mut map_perm = MapPermission::U;
    //             let ph_flags = ph.flags();
    //             if ph_flags.is_read() {
    //                 map_perm |= MapPermission::R;
    //             }
    //             if ph_flags.is_write() {
    //                 map_perm |= MapPermission::W;
    //             }
    //             if ph_flags.is_execute() {
    //                 map_perm |= MapPermission::X;
    //             }
    //             let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
    //             max_end_vpn = map_area.vpn_range.get_end();
    //             memory_set.push(
    //                 map_area,
    //                 Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
    //             );
    //         }
    //     }
    //     let max_end_va: VirtAddr = max_end_vpn.into();
    //     let mut user_stack_base: usize = max_end_va.into();
    //     user_stack_base += PAGE_SIZE;
    //     (
    //         memory_set,
    //         user_stack_base,
    //         elf.header.pt2.entry_point() as usize,
    //     )
    // }

    pub fn from_existed_user(user_space: &MemorySet, allocate_frame: FrameAllocFn) -> Self {
        let mut memory_set = Self::new_bare(allocate_frame);
        // map trampoline
        memory_set.map_trampoline();
        // copy data sections/trap_context/user_stack
        for area in user_space.areas.iter() {
            let new_area = MapArea::from_another(area);
            memory_set.push(new_area, None);
            // copy data from another space
            for vpn in area.vpn_range {
                let src_ppn = user_space.translate(vpn).unwrap().ppn();
                let dst_ppn = memory_set.translate(vpn).unwrap().ppn();
                dst_ppn
                    .get_bytes_array()
                    .copy_from_slice(src_ppn.get_bytes_array());
            }
        }
        memory_set
    }

    pub fn activate(&self) {
        let asid = 0;
        let ppn = self.page_table.root_ppn().into();

        unsafe {
            riscv::register::satp::set(satp::Mode::Sv39, asid, ppn);
            riscv::asm::sfence_vma(0, 0);
        }
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
    pub fn recycle_data_pages(&mut self) {
        //*self = Self::new_bare();
        self.areas.clear();
    }
}

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
    allocate_frame: FrameAllocFn,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
        allocate_frame: FrameAllocFn,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            allocate_frame,
        }
    }
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.get_start(), another.vpn_range.get_end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            allocate_frame: another.allocate_frame,
        }
    }
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = (self.allocate_frame)().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
            MapType::Linear(pn_offset) => {
                // check for sv39
                assert!(vpn.0 < (1usize << 27));
                ppn = PhysPageNum((vpn.0 as isize + pn_offset) as usize);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }

    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
    /// offset of page num
    Linear(isize),
}

bitflags! {
    #[derive(Copy, Clone)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

// #[allow(unused)]
// pub fn remap_test() {
//     let mut kernel_space = KERNEL_SPACE
//         .get()
//         .expect("KERNEL_SPACE not initialized")
//         .lock();

//     let mid_text: VirtAddr = ((TEXT_START() + TEXT_END()) / 2).into();
//     let mid_rodata: VirtAddr = ((RODATA_START() + RODATA_END()) / 2).into();
//     let mid_data: VirtAddr = ((DATA_START() + DATA_END()) / 2).into();
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_text.floor())
//         .unwrap()
//         .writable(),);
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_rodata.floor())
//         .unwrap()
//         .writable(),);
//     assert!(!kernel_space
//         .page_table
//         .translate(mid_data.floor())
//         .unwrap()
//         .executable(),);
//     println!("remap_test passed!");
// }
