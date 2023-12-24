use core::{
    cell::{Cell, RefCell, UnsafeCell},
    panic,
};

use alloc::{boxed::Box, sync::Arc};
use bootloader::BootInfo;
use lazy_static::lazy_static;
use spin::{Mutex, RwLock, RwLockWriteGuard, MutexGuard};
use x86_64::{structures::paging::OffsetPageTable, VirtAddr};

use crate::{drivers::{
    memory::{handler::MemoryHandler, rsdp::DescriptorTablesHandler}, self, fs::fs_driver::FsDriver, disk::ata::{DiskLoc, Channel, Drive},
}, serial_print, serial_println};

pub static STATE: RwLock<Kernel> = RwLock::new(Kernel::new());

pub struct Kernel {
    mem_handler: Option<Mutex<MemoryHandler>>,
    pub boot_info: Option<&'static BootInfo>,
    descriptor_tables: Option<Mutex<DescriptorTablesHandler>>,
    fs: Option<Mutex<FsDriver>>,
}
impl Kernel {
    pub const fn new() -> Self {
        Self {
            mem_handler: None,
            boot_info: None,
            descriptor_tables: None,
            fs: None,
        }
    }
    pub fn init(&mut self, boot_info: &'static bootloader::BootInfo) {
        self.boot_info.replace(boot_info);
        serial_print!("Memory");
        let mut mem_handler = MemoryHandler::new(
            boot_info.physical_memory_offset,
            &boot_info.memory_map,
        );
        serial_print!("Interrupts");
        drivers::interrupts::init();
        serial_print!("Ata");
        drivers::disk::ata::init();
        // self.descriptor_tables.replace(Mutex::new(DescriptorTablesHandler::new(
        //     &mut mem_handler,
        //     boot_info.physical_memory_offset,
        // )));
        self.mem_handler.replace(Mutex::new(mem_handler));
        serial_println!("Fs");
        self.fs.replace(Mutex::new(FsDriver::new(DiskLoc(Channel::Primary, Drive::Slave))));
    }
    pub fn mem_handler(&mut self) -> &mut Mutex<MemoryHandler> {
        self.mem_handler.as_mut().unwrap()
    }
    pub fn fs(&mut self) -> &mut Mutex<FsDriver> {
        self.fs.as_mut().unwrap()
    }
}
// don't use before kernel init
pub fn mem_handler() -> u64 {
    todo!()
}
pub fn fs_driver() -> u64 {
    todo!()
}
pub fn get_boot_info() -> &'static BootInfo {
    get_state().boot_info.unwrap()
}
pub fn get_state<'a>() -> RwLockWriteGuard<'a, Kernel> {
    STATE.write()
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
