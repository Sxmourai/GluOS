use bootloader::bootinfo::MemoryMap;
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

use super::{active_level_4_table, frame_allocator::BootInfoFrameAllocator};

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    pub fn new(physical_memory_offset: VirtAddr, memory_map: &'static MemoryMap) -> Self {
        let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };

        // let mut mapper = unsafe { x86_64::structures::paging::MappedPageTable::new(level_4_table, MyPageTableFrameMapping{next:0}) };
        let mut mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        let mut frame_allocator = unsafe {
            BootInfoFrameAllocator::init(memory_map) // Initialize the frame allocator
        };
        crate::allocator::init_heap(&mut mapper, &mut frame_allocator)
            .expect("heap initialization failed"); // Initialize the heap allocator
        Self {
            mapper,
            frame_allocator,
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
