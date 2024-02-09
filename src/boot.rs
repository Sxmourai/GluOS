use alloc::string::ToString;
use log::info;
use spin::Mutex;

use crate::{
    task::{executor::Executor, Task},
    user::shell::Shell,
};

pub fn boot(boot_info: &'static bootloader::BootInfo) -> Executor {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    unsafe { crate::state::BOOT_INFO.replace(boot_info) };
    crate::drivers::init_drivers();

    let mut executor = Executor::new();
    info!("Initialising shell");
    // executor.spawn(Task::new(QemuIOReader::new().run()));
    executor.spawn(Task::new(Shell::default().run_with_command("exec 10/userland.o".to_string())));
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
