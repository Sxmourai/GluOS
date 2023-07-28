use core::ffi::c_uchar;

use alloc::{vec::{Vec, self}, string::String, format};
use x86_64::{VirtAddr, structures::paging::{PhysFrame, Mapper, Page, PageTableFlags, Size4KiB}, PhysAddr};

use crate::{
    gdt, interrupts, serial_println, state::{self, STATE},
    task::executor::Executor,
    writer::{inb, outb, print_at},
    MemoryHandler, memory::{read_phys_memory_and_map, rsdp::DescriptorTablesHandler, self}, serial_print,
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;


// Boot the os, with the help of the 'boot_info' provided by the bootloader crate
pub fn boot(boot_info: &'static bootloader::BootInfo) {
    gdt::init();
    interrupts::init();

    let memory_handler = MemoryHandler::new(
        VirtAddr::new(boot_info.physical_memory_offset),
        &boot_info.memory_map,
    );
    unsafe {
        state::STATE.mem_handler = Some(memory_handler);
        state::STATE.boot_info = Some(boot_info);
    };
    let dth = DescriptorTablesHandler::new(boot_info.physical_memory_offset);
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
