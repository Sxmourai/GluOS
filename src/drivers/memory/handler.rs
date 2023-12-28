use bootloader::bootinfo::MemoryMap;
use x86_64::{structures::paging::{OffsetPageTable, Mapper, Page, Size4KiB, PhysFrame, PageTableFlags}, VirtAddr};

use log::trace;

use super::{active_level_4_table, frame_allocator::BootInfoFrameAllocator};

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    pub fn init_heap_and_frame_allocator(physical_memory_offset: u64, memory_map: &'static MemoryMap) -> Self {
        let physical_memory_offset = VirtAddr::new(physical_memory_offset);
        // trace!("Getting active level 4 table");
        let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

        // let mut mapper = unsafe { x86_64::structures::paging::MappedPageTable::new(level_4_table, MyPageTableFrameMapping{next:0}) };
        // trace!("Creating new memory mapper");
        let mut mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        // trace!("Creating new frame allocator");
        let mut frame_allocator = unsafe {
            BootInfoFrameAllocator::init(memory_map) // Initialize the frame allocator
        };
        crate::drivers::memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed"); // Initialize the heap allocator
        trace!("Finished initializing heap, can now begin tracing !");
        Self {
            mapper,
            frame_allocator,
        }
    }
    pub fn map_to(&mut self, page: Page<Size4KiB>, phys_frame: PhysFrame, flags: PageTableFlags){
        unsafe {
            self
                .mapper
                .map_to(page, phys_frame, flags, &mut self.frame_allocator)
                .unwrap()
                .flush()
        }
    }

    // pub fn frame_allocator(&mut self) -> &mut BootInfoFrameAllocator {
    //     serial_println!("{:?}",self.frame_allocator);
    //     Arc::clone(self.frame_allocator.as_mut().unwrap())
    // }
    // pub fn mapper(&mut self) -> Arc<Mutex<OffsetPageTable<'static>>> {
    //     Arc::clone(self.mapper.as_mut().unwrap())
    // }
}
