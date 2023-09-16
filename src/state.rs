use core::{
    cell::{Cell, RefCell},
    panic,
};

use alloc::{boxed::Box, sync::Arc};
use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

use crate::drivers::{memory::{handler::MemoryHandler, rsdp::DescriptorTablesHandler}, DriverManager};


pub static mut STATE: Kernel = Kernel::new();
pub struct Kernel {
    pub mem_handler: Option<Cell<MemoryHandler>>,
    pub boot_info: Option<&'static BootInfo>,
    pub descriptor_tables: Option<Mutex<DescriptorTablesHandler>>,
    pub driver_manager: Option<Cell<DriverManager>>,
}
impl Kernel {
    pub const fn new() -> Self {
        Self {
            mem_handler: None,
            boot_info: None,
            descriptor_tables: None,
            driver_manager: None,
        }
    }
}
// NOT USE BEFORE KERNEL INIT !!!
pub fn get_mem_handler() -> &'static mut Cell<MemoryHandler> {
    unsafe { STATE.mem_handler.as_mut().unwrap() }
}
pub fn get_boot_info() -> &'static BootInfo {
    unsafe { STATE.boot_info.unwrap() }
}
pub fn get_driver_manager() -> &'static mut Cell<DriverManager> {
    unsafe { STATE.driver_manager.as_mut().unwrap() }
}


/*
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
*/
