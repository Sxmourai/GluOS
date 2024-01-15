use bootloader::bootinfo::MemoryMap;
use x86_64::{
    structures::paging::{Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame, Size4KiB, FrameAllocator},
    VirtAddr,
};

use log::trace;

use crate::boot_info;

use super::{active_level_4_table, frame_allocator::BootInfoFrameAllocator};


pub fn init() {
    let off = unsafe{boot_info!()}.physical_memory_offset;
    let mem_handler = MemoryHandler::new(off, &unsafe{boot_info!()}.memory_map);
    unsafe { crate::state::MEM_HANDLER.replace(mem_handler) };
}

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    /// Inits heap & frame allocator
    pub fn new(
        physical_memory_offset: u64,
        memory_map: &'static MemoryMap,
    ) -> Self {
        let physical_memory_offset = VirtAddr::new(physical_memory_offset);
        // trace!("Getting active level 4 table");
        let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

        let mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        let frame_allocator = unsafe {
            BootInfoFrameAllocator::init(memory_map)
        };
        let mut _self = Self {
            mapper,
            frame_allocator,
        };
        crate::drivers::memory::allocator::init_heap(&mut _self)
            .expect("heap initialization failed"); // Initialize the heap allocator
        trace!("Finished initializing heap, can now begin tracing !");
        _self
    }
    pub unsafe fn map_to(&mut self, page: Page<Size4KiB>, phys_frame: PhysFrame, flags: PageTableFlags) {
        unsafe {
            self.mapper
                .map_to(page, phys_frame, flags, &mut self.frame_allocator)
                .unwrap()
                .flush()
        }
    }
    pub unsafe fn map(&mut self, page: Page<Size4KiB>, flags: PageTableFlags) -> Result<(), MapFrameError> {
        let frame = self.frame_allocator.allocate_frame();
        if frame.is_none() {return Err(MapFrameError::CantAllocateFrame)}
        let frame = frame.unwrap();
        unsafe {
            self.mapper
                .map_to(page, frame, flags, &mut self.frame_allocator)
                .unwrap()
                .flush()
        }
        Ok(())
    }

    // pub fn frame_allocator(&mut self) -> &mut BootInfoFrameAllocator {
    //     serial_println!("{:?}",self.frame_allocator);
    //     Arc::clone(self.frame_allocator.as_mut().unwrap())
    // }
    // pub fn mapper(&mut self) -> Arc<Mutex<OffsetPageTable<'static>>> {
    //     Arc::clone(self.mapper.as_mut().unwrap())
    // }
}

pub enum MapFrameError {
    CantAllocateFrame
}