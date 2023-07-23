use core::{
    cell::{Cell, RefCell},
    panic,
};

use alloc::{boxed::Box, sync::Arc};
use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

use crate::memory::BootInfoFrameAllocator;
use crate::MemoryHandler;

// const MEM_HANDLER

pub static mut STATE: Kernel = Kernel::new();
pub struct Kernel {
    pub mem_handler: Option<MemoryHandler>,
    pub boot_info: Option<&'static BootInfo>,
}
impl Kernel {
    pub const fn new() -> Self {
        Self {
            mem_handler: None,
            boot_info: None,
        }
    }
    pub fn get_mem_handler(&mut self) -> Cell<&mut MemoryHandler> {
        Cell::new(self.mem_handler.as_mut().unwrap())
    }
    pub fn boot_info(&self) -> &'static BootInfo {
        self.boot_info.unwrap()
    }
}
// NOT USE BEFORE KERNEL INIT !!!
pub fn get_mem_handler() -> Cell<&'static mut MemoryHandler> {
    unsafe { STATE.get_mem_handler() }
}
// pub fn get_mapper() -> Arc<Mutex<OffsetPageTable<'static>>> {
//     get_mem_handler().mapper()
// }
// pub fn get_frame_allocator() -> Arc<BootInfoFrameAllocator> {
//     get_mem_handler().frame_allocator()
// }
pub fn get_boot_info() -> &'static BootInfo {
    unsafe { STATE.boot_info() }
}

trait InKernel: Send {
    fn get_memory_handler(self: Box<Self>) -> MemoryHandler;
    fn get_boot_info(&self) -> &'static BootInfo;
}

struct InnerKernel {
    pub memory_handler: MemoryHandler,
    pub boot_info: &'static BootInfo,
}
impl InKernel for InnerKernel {
    fn get_memory_handler(self: Box<Self>) -> MemoryHandler {
        self.memory_handler
    }
    fn get_boot_info(&self) -> &'static BootInfo {
        self.boot_info
    }
}
struct DummyInKernel; // Cheating on the borrow checker ^^
impl InKernel for DummyInKernel {
    fn get_memory_handler(self: Box<Self>) -> MemoryHandler {
        panic!("Dummy kernel can't return app state !")
    }
    fn get_boot_info(&self) -> &'static BootInfo {
        panic!("Dummy kernel can't return app state !")
    }
}
