use alloc::string::ToString;
use log::info;
use spin::Mutex;

use crate::{
    drivers::{
        self,
        disk::ata::{Channel, DiskLoc, Drive},
        memory::handler::MemoryHandler,
    }, memory::tables::DescriptorTablesHandler, pit::{sdelay, udelay}, state::{self, MEM_HANDLER}, task::{executor::Executor, Task}, user::{self, shell::Shell}
};

pub fn boot(boot_info: &'static bootloader::BootInfo) -> Executor {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    unsafe { state::BOOT_INFO.replace(boot_info) };
    drivers::init_drivers();

    let mut executor = Executor::new();
    info!("Initialising shell");
    executor.spawn(Task::new(Shell::default().run_with_command("".to_string())));
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
