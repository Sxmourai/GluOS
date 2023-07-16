use x86_64::VirtAddr;

use crate::{
    gdt, interrupts,
    task::executor::Executor, memory::MemoryHandler, state,
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

// Boot the os, with the help of the 'boot_info' provided by the bootloader crate
pub fn boot(boot_info: &'static bootloader::BootInfo) {
    gdt::init();
    interrupts::init();
    let memory_handler = MemoryHandler::new(VirtAddr::new(boot_info.physical_memory_offset), &boot_info.memory_map);
    unsafe {
        state::STATE.mem_handler = Some(memory_handler);
        state::STATE.boot_info = Some(boot_info);
    };
 }



pub fn end() -> ! {
    let mut executor = Executor::new();
    // executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}