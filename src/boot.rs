use core::{cell::Cell, ffi::c_uchar};

use ::log::{debug, trace};
use alloc::{
    format,
    string::String,
    vec::{self, Vec},
};
use spin::Mutex;
use x86_64::{
    structures::paging::{Mapper, Page, PageTableFlags, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::{
    drivers::{self, memory::rsdp::DescriptorTablesHandler},
    log::{self, initialize_logger},
    serial_print, serial_println,
    state::{self, STATE, get_state},
    task::executor::Executor,
    writer::{inb, outb, print_at},
    Kernel,
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

// Boot the os, with the help of the 'boot_info' provided by the bootloader crate
pub fn boot(boot_info: &'static bootloader::BootInfo) {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    drivers::gdt::init();
    get_state().init(boot_info);
    initialize_logger();
    serial_println!("\t[Done booting]\n");
    println!("Finished booting");
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
