use bootloader::bootinfo::MemoryMap;
use x86_64::{
    structures::paging::{
        mapper::{MapToError, MapperFlush, UnmapError},
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
    unsafe { crate::state::MEM_HANDLER.replace(mem_handler); }
}

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    /// Inits heap & frame allocator
    #[must_use] pub fn new(physical_memory_offset: u64, memory_map: &'static MemoryMap) -> Self {
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
    /// Safe ? Wrapper around allocating frames and mapping them
    pub fn alloc(&mut self, flags: PageTableFlags) -> Result<PhysFrame, MapToError<Size4KiB>> {
        let frame = self.frame_allocator.allocate_frame().ok_or(MapToError::FrameAllocationFailed)?;
        let page = Page::from_start_address(VirtAddr::new(frame.start_address().as_u64())).unwrap();
        unsafe{self.map_frame(page, frame, flags)?};
        Ok(frame)
    }
    /// # Safety
    /// Mapping can cause all sorts of panics, set `OffsetPageTable`
    pub unsafe fn map(
        &mut self,
        page: Page<Size4KiB>,
        flags: PageTableFlags,
    ) -> Result<PhysAddr, MapToError<Size4KiB>> {
        let frame = self.frame_allocator.allocate_frame();
        if frame.is_none() {
            return Err(MapToError::FrameAllocationFailed);
        }
        let frame = frame.unwrap();
        unsafe { self.map_frame(page, frame, flags)? }
        Ok(frame.start_address())
    }
    /// # Safety
    /// Mapping can cause all sorts of panics, set `OffsetPageTable`
    pub unsafe fn unmap(
        &mut self,
        page: Page<Size4KiB>,
    ) -> Result<(PhysFrame, MapperFlush<Size4KiB>), UnmapError> {
        unsafe { self.mapper.unmap(page) }
    }

    /// # Safety
    /// Mapping can cause all sorts of panics, set `OffsetPageTable`
    pub unsafe fn map_frame(
        &mut self,
        page: Page<Size4KiB>,
        frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<(), MapToError<Size4KiB>> {
        unsafe {
            self.mapper
                .map_to(page, frame, flags, &mut self.frame_allocator)?
                .flush();
        }
        Ok(())
    }
    pub fn malloc(&mut self, flags: PageTableFlags) -> Option<VirtAddr> {
        let frame = self.frame_allocator.allocate_frame()?;
        let virt_addr = VirtAddr::new(frame.start_address().as_u64());
        let page = Page::from_start_address(virt_addr).ok()?;
        unsafe { self.map_frame(page, frame, flags) }.ok()?;
        Some(virt_addr)
    }
}
/// Unsafe not set for ease of use... Maybe change that
/// TODO Do we want to keep this function not unsafe even though it is ?
#[track_caller]
#[must_use] pub fn map(page: Page<Size4KiB>, flags: PageTableFlags) -> PhysAddr {
    unsafe { mem_handler!().map(page, flags) }.unwrap()
} // TODO Refactor those functions
#[track_caller]
pub fn map_frame(page: Page<Size4KiB>, frame: PhysFrame, flags: PageTableFlags) {
    match unsafe { mem_handler!().map_frame(page, frame, flags) } {
        Ok(()) => {},
        Err(err) => match err {
            MapToError::FrameAllocationFailed => todo!(),
            MapToError::ParentEntryHugePage => todo!(),
            MapToError::PageAlreadyMapped(already_frame) => {
                log::trace!("Tried to map page at {:#x} -> {:#x}({:?}) but it's already mapped to {:#x}", page.start_address(), frame.start_address(), flags, already_frame.start_address());
                unsafe{mem_handler!().unmap(page).unwrap()}; // Could use update_flags ?
                unsafe { mem_handler!().map_frame(page, frame, flags) }.unwrap();
            }, // If the page is already mapped, it like nothing happened, so we don't need to panic
        },
    }
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
    () => {
        $crate::mem_handler!().malloc(PageTableFlags::WRITABLE | PageTableFlags::PRESENT)
    };
}

#[derive(Debug)]
pub enum MapFrameError {
    CantAllocateFrame,
}
