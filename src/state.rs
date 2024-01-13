use bootloader::BootInfo;
use spin::{Mutex, RwLock, RwLockWriteGuard};

use crate::{drivers::{
    fs::fs_driver::FsDriver,
    memory::handler::MemoryHandler,
}, memory::tables::DescriptorTablesHandler};

pub static STATE: RwLock<Kernel> = RwLock::new(Kernel::new());

pub struct Kernel {
    mem_handler: Option<Mutex<MemoryHandler>>,
    pub boot_info: Option<&'static BootInfo>,
    #[allow(unused)]
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
    pub fn init(
        &mut self,
        boot_info: &'static bootloader::BootInfo,
        mem_handler: MemoryHandler,
        fs_driver: FsDriver,
    ) {
        self.boot_info.replace(boot_info);
        // self.descriptor_tables.replace(Mutex::new(DescriptorTablesHandler::new(
        //     &mut mem_handler,
        //     boot_info.physical_memory_offset,
        // )));
        self.mem_handler.replace(Mutex::new(mem_handler));
        self.fs.replace(Mutex::new(fs_driver));
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
