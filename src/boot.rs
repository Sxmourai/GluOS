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
    drivers::{self, memory::handler::MemoryHandler, fs::fs_driver::FsDriver, disk::ata::{DiskLoc, Channel, Drive}},
    serial_println,
    state::get_state,
    state::Kernel, video, user::{shell::Shell, self}, task::executor::Executor,
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

pub fn boot(boot_info: &'static bootloader::BootInfo) {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    drivers::gdt::init();
    let mut mem_handler = MemoryHandler::init_heap_and_frame_allocator(
        boot_info.physical_memory_offset,
        &boot_info.memory_map,
    );
    drivers::interrupts::init();
    drivers::disk::ata::init();
    drivers::time::init();
    drivers::video::init_graphics();
    let fs_driver = FsDriver::new(DiskLoc(Channel::Primary, Drive::Slave));
    get_state().init(boot_info, mem_handler, fs_driver);
    user::log::initialize_logger();
    serial_println!("\t[Done booting]\n");
    get_state().fs().lock().write_dir("");
    Shell::new();
}

pub fn end() -> ! { //TODO Implement async stuff & all in Executor
    // hlt_loop()
    let mut executor = Executor::new();
    // // executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
