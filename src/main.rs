#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_runner"]

extern crate alloc;
extern crate bootloader;
extern crate kernel;
extern crate x86_64;

use crate::kernel::{hlt_loop, serial_println};
use alloc::vec::Vec;
use bootloader::{entry_point, BootInfo};
use core::{
    ffi::{c_uchar, c_ushort},
    panic::PanicInfo,
};
use hashbrown::HashMap;
use kernel::{
    println,
    prompt::input,
    serial_print,
    state::get_mem_handler,
    terminal::{
        console::{pretty_print, CONSOLE},
        shell::Shell,
    },
    writer::{inb, outb, outb16}, pci::pci_data::print_all_pci_devices,
};
use pci_ids::SubSystem;
use x86_64::{instructions::hlt, VirtAddr};

// Sets the entry point of our kernel for the bootloader. This means we can have the 'boot_info' variable which stores some crucial info
entry_point!(kernel_main);
// Main function of our kernel (1 func to start when boot if not in test mode). Never returns, because kernel runs until machine poweroff
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize & boot the device and kernel
    kernel::boot(boot_info);
    print_all_pci_devices();
    // unsafe { kernel::pci::ata::initialize_sata_controller() };
    // Shell::new();

    hlt_loop()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC:{}", info);
    kernel::hlt_loop();
}
