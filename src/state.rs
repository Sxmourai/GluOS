use core::panic;

use alloc::{boxed::Box, sync::Arc};
use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{VirtAddr, structures::paging::OffsetPageTable};

use crate::memory::{MemoryHandler, BootInfoFrameAllocator};

lazy_static!{pub static ref STATE: Mutex<Kernel> = Mutex::new(Kernel::new());}

pub struct Kernel {
    pub inner: Box<dyn InKernel>,
}
impl Kernel {
    pub fn new() -> Self {
        Self {
            inner: Box::new(DummyInKernel {})
        }
    }
    pub fn init(&mut self, boot_info: &'static BootInfo) {
        crate::boot::init();
        self.inner = Box::new(InnerKernel {
            memory_handler: MemoryHandler::new(VirtAddr::new(boot_info.physical_memory_offset), &boot_info.memory_map),
            boot_info
        });
    }
    // pub fn get_mem_handler(self) -> MemoryHandler {
    //     self.inner.get_memory_handler()
    // }
    pub fn boot_info(&self) -> &'static BootInfo {
        self.inner.get_boot_info()
    }
}
// NOT USE BEFORE KERNEL INIT !!!
pub fn get_mem_handler() -> MemoryHandler {
    STATE.lock().inner.get_memory_handler()
}
// pub fn get_mapper() -> Arc<Mutex<OffsetPageTable<'static>>> {
//     get_mem_handler().mapper()
// }
// pub fn get_frame_allocator() -> Arc<BootInfoFrameAllocator> {
//     get_mem_handler().frame_allocator()
// }
pub fn get_boot_info() -> &'static BootInfo {
    STATE.lock().boot_info()
}

trait InKernel : Send {
    fn get_memory_handler(self) -> MemoryHandler;
    fn get_boot_info(&self) -> &'static BootInfo;
}

struct InnerKernel {
    pub memory_handler: MemoryHandler,
    pub boot_info: &'static BootInfo
}
impl InKernel for InnerKernel {
    fn get_memory_handler(self) -> MemoryHandler {self.memory_handler}
    fn get_boot_info(&self) -> &'static BootInfo {self.boot_info}
}
struct DummyInKernel; // Cheating on the borrow checker ^^
impl InKernel for DummyInKernel {
    fn get_memory_handler(self) -> MemoryHandler {panic!("Dummy kernel can't return app state !")}
    fn get_boot_info(&self) -> &'static BootInfo {panic!("Dummy kernel can't return app state !")}
}