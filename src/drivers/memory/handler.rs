use bootloader::bootinfo::MemoryMap;
use x86_64::{
    structures::paging::{
        mapper::{MapperFlush, UnmapError},
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

use log::trace;

use crate::{boot_info, mem_handler};

use super::{active_level_4_table, frame_allocator::BootInfoFrameAllocator};

/// Initialises the heap allocator and the memory paging driver
pub fn init() {
    let off = unsafe { boot_info!() }.physical_memory_offset;
    let mem_handler = MemoryHandler::new(off, &unsafe { boot_info!() }.memory_map);
    unsafe { crate::state::MEM_HANDLER.replace(mem_handler) };
}

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    /// Inits heap & frame allocator
    pub fn new(physical_memory_offset: u64, memory_map: &'static MemoryMap) -> Self {
        let physical_memory_offset = VirtAddr::new(physical_memory_offset);
        // trace!("Getting active level 4 table");
        let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

        let mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        let frame_allocator = unsafe { BootInfoFrameAllocator::init(memory_map) };
        let mut _self = Self {
            mapper,
            frame_allocator,
        };
        crate::drivers::memory::allocator::init_heap(&mut _self)
            .expect("heap initialization failed"); // Initialize the heap allocator
        trace!("Finished initializing heap, can now begin tracing !");
        _self
    }
    /// # Safety
    /// Mapping can cause all sorts of panics, set OffsetPageTable
    pub unsafe fn map(
        &mut self,
        page: Page<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<PhysAddr, MapFrameError> {
        let frame = self.frame_allocator.allocate_frame();
        if frame.is_none() {
            return Err(MapFrameError::CantAllocateFrame);
        }
        let frame = frame.unwrap();
        unsafe { self.map_frame(page, frame, flags)? }
        Ok(frame.start_address())
    }
    /// # Safety
    /// Mapping can cause all sorts of panics, set OffsetPageTable
    pub unsafe fn unmap(
        &mut self,
        page: Page<Size4KiB>,
    ) -> Result<(PhysFrame, MapperFlush<Size4KiB>), UnmapError> {
        unsafe { self.mapper.unmap(page) }
    }

    /// # Safety
    /// Mapping can cause all sorts of panics, set OffsetPageTable
    pub unsafe fn map_frame(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<(), MapFrameError> {
        unsafe {
            self.mapper
                .map_to(page, frame, flags, &mut self.frame_allocator)
                .map_err(|err| MapFrameError::CantAllocateFrame)?
                .flush()
        }
        Ok(())
    }
    pub fn malloc(&mut self, flags: PageTableFlags) -> Option<VirtAddr> {
        let frame = self.frame_allocator.allocate_frame()?;
        let virt_addr = VirtAddr::new(frame.start_address().as_u64());
        let page = Page::from_start_address(virt_addr).ok()?;
        unsafe{self.map_frame(page, frame, flags)}.ok()?;
        Some(virt_addr)
    }

}
///TODO Is it unsafe ?
pub fn map(page: Page<Size4KiB>, flags: PageTableFlags) -> PhysAddr {
    unsafe { mem_handler!().map(page, flags) }.unwrap()
}
pub fn map_frame(page: Page<Size4KiB>, frame: PhysFrame, flags: PageTableFlags) {
    unsafe { mem_handler!().map_frame(page, frame, flags) }.unwrap()
}
#[macro_export]
macro_rules! mem_map {
    (frame_addr=$addr: expr, $($arg: tt)*) => {
        let page = x86_64::structures::paging::Page::containing_address(x86_64::VirtAddr::new($addr));
        let frame = x86_64::structures::paging::PhysFrame::containing_address(x86_64::PhysAddr::new($addr));
        $crate::mem_map!(page, frame=frame, $($arg)*);
    };
    ($page: expr, frame=$frame: expr, WRITABLE) => {
        let flags = x86_64::structures::paging::PageTableFlags::PRESENT | x86_64::structures::paging::PageTableFlags::WRITABLE;
        $crate::mem_map!($page,frame=$frame, flags);
    };
    ($page: expr, frame=$frame: expr, $flags: expr) => {
        if unsafe{$crate::mem_handler!().map_frame($page,$frame,$flags)}.is_err() {
            log::error!("Failed mapping {:?} -> {:?} with flags: {:#b}", $page, $frame, $flags);
        }
    };
    ($page: expr, $flags: expr) => {
        if unsafe{$crate::mem_handler!().map($page,$flags)}.is_err() {
            log::error!("Failed mapping {:?} with flags: {:#b}", $page, $flags);
        }
    };
}

#[macro_export]
macro_rules! malloc {
    ($flags: expr) => {
        $crate::mem_handler!().malloc($flags)
    };
}

#[derive(Debug)]
pub enum MapFrameError {
    CantAllocateFrame,
}
