use core::cell::Cell;

use bootloader::BootInfo;
use spin::{Mutex, RwLock, RwLockWriteGuard};

use crate::{drivers::memory::handler::MemoryHandler, memory::tables::DescriptorTablesHandler};

pub static mut BOOT_INFO: Option<&'static bootloader::BootInfo> = None;
pub static mut MEM_HANDLER: Option<MemoryHandler> = None;
#[cfg(feature="fs")]
pub static mut FS_DRIVER: Option<FsDriverManager> = None;
pub static mut DESCRIPTOR_TABLES: Option<DescriptorTablesHandler> = None;


// don't use before kernel init
#[macro_export]
macro_rules! boot_info {
    () => {
        &mut crate::state::BOOT_INFO.as_mut().unwrap()
    };
}

#[macro_export]
macro_rules! mem_handler {
    () => {
        unsafe{crate::state::MEM_HANDLER.as_mut().unwrap()}
    };
}

#[macro_export]
macro_rules! fs_driver {
    () => {
        unsafe{crate::state::FS_DRIVER.as_mut().unwrap()}
    };
}

#[macro_export]
macro_rules! descriptor_tables {
    () => {
        unsafe{crate::state::DESCRIPTOR_TABLES.as_mut().unwrap()}
    };
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
