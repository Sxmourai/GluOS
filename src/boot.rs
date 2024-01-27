use alloc::string::ToString;
use log::info;
use spin::Mutex;

use crate::{
    drivers::{
        self,
        disk::ata::{Channel, DiskLoc, Drive},
        memory::handler::MemoryHandler,
    },
    task::{executor::Executor, Task},
    user::{self, shell::Shell}, memory::tables::DescriptorTablesHandler, state::{self, MEM_HANDLER},
};

pub fn boot(boot_info: &'static bootloader::BootInfo) -> Executor {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    unsafe { state::BOOT_INFO.replace(boot_info) };
    drivers::init_drivers();
    
    let mut executor = Executor::new();
    info!("Initialising shell");
    executor.spawn(Task::new(Shell::new().run_with_command("read 30/".to_string())));
    // executor.spawn(Task::new());
    executor
}

pub fn end(mut executor: Executor) -> ! {
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
