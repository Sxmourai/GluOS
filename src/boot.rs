use core::{ffi::c_uchar, cell::Cell};

use alloc::{vec::{Vec, self}, string::String, format};
use ::log::trace;
use spin::Mutex;
use x86_64::{VirtAddr, structures::paging::{PhysFrame, Mapper, Page, PageTableFlags, Size4KiB}, PhysAddr};

use crate::{
    serial_println, state::{self, STATE},
    task::executor::Executor,
    writer::{inb, outb, print_at},
    serial_print, log::{self, initialize_logger}, drivers::{self, memory::rsdp::DescriptorTablesHandler},
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;


// Boot the os, with the help of the 'boot_info' provided by the bootloader crate
pub fn boot(boot_info: &'static bootloader::BootInfo) {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    unsafe {
        state::STATE.boot_info = Some(boot_info);
    };
    drivers::init();
    initialize_logger();
    trace!("Initializing GDT");
    trace!("Initializing Interrupts & CPU exceptions");
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
