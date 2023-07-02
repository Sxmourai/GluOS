#![no_std]
#![no_main]

extern crate kernel;
extern crate bootloader;
extern crate x86_64;
extern crate alloc;

use core::panic::PanicInfo;
use kernel::println;
use bootloader::{BootInfo, entry_point};
use alloc::{boxed::Box, rc::Rc, vec::Vec};
use kernel::task::keyboard;
use kernel::{memory::{self, BootInfoFrameAllocator}, allocator};
use x86_64::VirtAddr;use alloc::vec;
use kernel::task::Task;

entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::init(boot_info);

    kernel::end();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    kernel::hlt_loop();
}

