use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{VirtAddr, structures::paging::OffsetPageTable};

use crate::memory::{MemoryHandler, BootInfoFrameAllocator};

lazy_static!{pub static ref state: Mutex<Kernel> = Mutex::new(Kernel::new());}

pub struct Kernel {
    pub memory_handler: Option<MemoryHandler>,
    pub boot_info: Option<&'static BootInfo>
}
impl Kernel {
    pub fn new() -> Self {
        Self {
            memory_handler: None,
            boot_info: None
        }
    }
    pub fn init(&mut self, boot_info: &'static BootInfo) {
        crate::boot::init();
        self.memory_handler = Some(MemoryHandler::new(VirtAddr::new(boot_info.physical_memory_offset), &boot_info.memory_map));
        self.boot_info = Some(boot_info);
    }
}
// NOT USE BEFORE KERNEL INIT !!!
pub fn get_mapper() -> &'static mut OffsetPageTable<'static> {
    state.lock().memory_handler.unwrap().mapper()
}
pub fn get_frame_allocator() -> &'static mut BootInfoFrameAllocator {
    state.lock().memory_handler.unwrap().frame_allocator()
}
pub fn get_mem_handler() -> &'static mut MemoryHandler {
    &mut state.lock().memory_handler.unwrap()
}
pub fn get_boot_info() -> &'static BootInfo {
    state.lock().boot_info.unwrap()
}
