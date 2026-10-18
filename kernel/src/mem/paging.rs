//! Sv39 uses three levels of 512 entries for 4 KiB, 2 MiB, and 1 GiB mappings.

use core::ops::{BitOr, BitOrAssign};
use riscv::{asm::sfence_vma, register::satp};

pub const PAGE_SIZE: usize = 4096;
const PPN_MASK: u64 = (1 << 44) - 1;

/// An Sv39 virtual address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtAddr(pub usize);

impl VirtAddr {
    fn is_valid(self, alignment: usize) -> bool {
        (self.0 as i64 >> 38 == 0 || self.0 as i64 >> 38 == -1) && self.0 & (alignment - 1) == 0
    }

    fn index(self, level: usize) -> usize {
        (self.0 >> (12 + level * 9)) & 0x1ff
    }

    fn offset(self, size: PageSize) -> usize {
        self.0 & (size.bytes() - 1)
    }
}

/// A physical address.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PhysAddr(pub usize);

impl PhysAddr {
    fn is_valid(self, alignment: usize) -> bool {
        self.0 >> 56 == 0 && self.0 & (alignment - 1) == 0
    }

    const fn page_number(self) -> u64 {
        (self.0 >> 12) as u64
    }
}

/// Size of a leaf mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageSize {
    Size4K,
    Size2M,
    Size1G,
}

impl PageSize {
    pub const fn bytes(self) -> usize {
        match self {
            Self::Size4K => 1 << 12,
            Self::Size2M => 1 << 21,
            Self::Size1G => 1 << 30,
        }
    }

    const fn level(self) -> usize {
        match self {
            Self::Size4K => 0,
            Self::Size2M => 1,
            Self::Size1G => 2,
        }
    }
}

/// Leaf permissions. `map` sets the accessed and dirty bits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flags(u64);

impl Flags {
    pub const READ: Self = Self(1 << 1);
    pub const WRITE: Self = Self(1 << 2);
    pub const EXECUTE: Self = Self(1 << 3);
    pub const USER: Self = Self(1 << 4);
    pub const GLOBAL: Self = Self(1 << 5);

    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for Flags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for Flags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    InvalidAddress,
    InvalidFlags,
    AlreadyMapped,
    OutOfFrames,
}

/// A physical address and the leaf permissions found during a page-table walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Translation {
    pub physical: PhysAddr,
    pub size: PageSize,
    pub flags: Flags,
}

/// An Sv39 page table. The caller owns its frames.
///
/// Dropping this value does not free frames. Flush after editing an active table.
pub struct PageTable {
    root: PhysAddr,
}

impl PageTable {
    /// Wrap an initialized root table.
    ///
    /// # Safety
    /// All table frames must be 4 KiB, identity-mapped, and exclusively owned
    /// until hardware stops using this table.
    pub unsafe fn new(root: PhysAddr) -> Self {
        assert!(root.is_valid(PAGE_SIZE));
        Self { root }
    }

    pub const fn root(&self) -> PhysAddr {
        self.root
    }

    /// Build the SATP value for this root and a 16-bit ASID.
    pub const fn satp(&self, asid: u16) -> u64 {
        (8_u64 << 60) | ((asid as u64) << 44) | self.root.page_number()
    }

    /// Switch SATP on the current hart and flush its TLB.
    ///
    /// # Safety
    /// The new table must map the current code and stack. Coordinate other harts.
    pub unsafe fn activate(&self, asid: u16) {
        unsafe {
            satp::set(
                satp::Mode::Sv39,
                asid as usize,
                self.root.page_number() as usize,
            )
        };
        self.flush();
    }

    /// Flush this hart's translations after editing an active table.
    /// Other harts need their own shootdown.
    pub fn flush(&self) {
        sfence_vma(0, 0);
    }

    /// Map one aligned leaf. Intermediate frames remain allocated after unmapping.
    ///
    /// # Safety
    /// `allocate` must return zeroed, exclusive 4 KiB frames that remain accessible
    /// while the table is in use.
    pub unsafe fn map(
        &mut self,
        virtual_address: VirtAddr,
        physical_address: PhysAddr,
        size: PageSize,
        flags: Flags,
        mut allocate: impl FnMut() -> Option<PhysAddr>,
    ) -> Result<(), MapError> {
        if !virtual_address.is_valid(size.bytes()) || !physical_address.is_valid(size.bytes()) {
            return Err(MapError::InvalidAddress);
        }
        if flags.0 & (Flags::READ.0 | Flags::EXECUTE.0) == 0
            || flags.contains(Flags::WRITE) && !flags.contains(Flags::READ)
        {
            return Err(MapError::InvalidFlags);
        }

        let mut table = self.root;
        for level in (size.level() + 1..3).rev() {
            let entry = self.entry(table, virtual_address, level);
            let value = unsafe { entry.read() };
            if value & 1 == 0 {
                let frame = allocate().ok_or(MapError::OutOfFrames)?;
                if !frame.is_valid(PAGE_SIZE) {
                    return Err(MapError::InvalidAddress);
                }
                unsafe { entry.write((frame.page_number() << 10) | 1) };
                table = frame;
            } else if value & 0xe != 0 {
                return Err(MapError::AlreadyMapped);
            } else {
                table = PhysAddr(((value >> 10) & PPN_MASK) as usize * PAGE_SIZE);
            }
        }

        let entry = self.entry(table, virtual_address, size.level());
        if unsafe { entry.read() } & 1 != 0 {
            return Err(MapError::AlreadyMapped);
        }
        // A/D are set in software so implementations do not need to fault to set them.
        unsafe { entry.write((physical_address.page_number() << 10) | flags.0 | 0xc1) };
        Ok(())
    }

    /// Remove a leaf only when its virtual address and size match the mapping.
    /// Returns its physical address and flags. Does not free frames or flush TLBs.
    pub fn unmap(&mut self, virtual_address: VirtAddr, size: PageSize) -> Option<Translation> {
        if !virtual_address.is_valid(size.bytes()) {
            return None;
        }
        let (entry, translation) = self.walk(virtual_address)?;
        if translation.size != size {
            return None;
        }
        unsafe { entry.write(0) };
        Some(translation)
    }

    /// Translate a virtual address without checking access privileges.
    pub fn translate(&self, virtual_address: VirtAddr) -> Option<Translation> {
        self.walk(virtual_address)
            .map(|(_, translation)| translation)
    }

    fn walk(&self, virtual_address: VirtAddr) -> Option<(*mut u64, Translation)> {
        if !virtual_address.is_valid(1) {
            return None;
        }
        let mut table = self.root;
        for level in (0..3).rev() {
            let entry = self.entry(table, virtual_address, level);
            let value = unsafe { entry.read() };
            if value & 1 == 0 {
                return None;
            }
            let physical = ((value >> 10) & PPN_MASK) as usize * PAGE_SIZE;
            if value & 0xe != 0 {
                let size = match level {
                    2 => PageSize::Size1G,
                    1 => PageSize::Size2M,
                    _ => PageSize::Size4K,
                };
                if physical & (size.bytes() - 1) != 0 {
                    return None;
                }
                return Some((
                    entry,
                    Translation {
                        physical: PhysAddr(physical | virtual_address.offset(size)),
                        size,
                        flags: Flags(value & 0x3e),
                    },
                ));
            }
            table = PhysAddr(physical);
        }
        None
    }

    fn entry(&self, table: PhysAddr, virtual_address: VirtAddr, level: usize) -> *mut u64 {
        unsafe { (table.0 as *mut u64).add(virtual_address.index(level)) }
    }
}
