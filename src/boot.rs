use alloc::string::ToString as _;

use crate::{
    get_drivers,
    task::{executor::Executor, Task},
    user::shell::Shell,
    DriverId,
};

#[must_use] pub fn boot(boot_info: &'static bootloader::BootInfo) -> Executor {
    unsafe { crate::state::BOOT_INFO.replace(boot_info); }
    crate::drivers::memory::init(); // Executor needs heap allocation
    let mut executor = Executor::new();
    for drv in get_drivers() {
        executor.spawn(drv.task);
    }
    executor.spawn(Task::new(async {log::info!("Done booting !")}));
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
