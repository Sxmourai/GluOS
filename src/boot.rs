use alloc::string::ToString;
use log::info;
use spin::Mutex;

use crate::{
    get_drivers,
    task::{executor::Executor, Task},
    user::shell::Shell,
    DriverId,
};

pub fn boot(boot_info: &'static bootloader::BootInfo) -> Executor {
    unsafe { crate::state::BOOT_INFO.replace(boot_info) };
    crate::drivers::memory::init(); // Executor needs heap allocation
    let mut executor = Executor::new();
    for drv in get_drivers() {
        executor.spawn(drv.task);
    }
    executor.spawn(Task::new(
        async {
            log::debug!("Finished booting");
            crate::time::sdelay(3);
            log::debug!("Finished booting");

        },
    ));
    executor.spawn(Task::new(
        Shell::default().run_with_command("exec 10/userland.o".to_string()),
    ));
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
