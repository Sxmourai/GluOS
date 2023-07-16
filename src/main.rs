#![no_std]
#![no_main]
#![feature(custom_test_frameworks)] // Required for ´cargo test´ because it searches in main.rs even if no tests
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(unused)] // Stop cargo warnings

extern crate kernel;
extern crate bootloader;
extern crate x86_64;
extern crate alloc;

use core::panic::PanicInfo;
use crate::kernel::{serial_println, hlt_loop};
use alloc::vec::Vec;
use bootloader::{BootInfo, entry_point};
use kernel::{serial_print, println, state::get_mem_handler};
use pci_ids::SubSystem;
use x86_64::VirtAddr;

// Sets the entry point of our kernel for the bootloader. This means we can have the 'boot_info' variable which stores some crucial info
entry_point!(kernel_main);
// Main function of our kernel (1 func to start when boot if not in test mode). Never returns, because kernel runs until machine poweroff
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize & boot the device and kernel
    kernel::boot(boot_info);

    serial_println!("{}", kernel::prompt::input("Command: "));

    // let to_print = unsafe { rsdp::Rsdp::search_for_on_bios(&mut kernel::state::get_mem_handler().get_mut().frame_allocator) }.unwrap();
    // crate::serial_println!("Phys start: {:?}", to_print.physical_start());

    // x86_64::PhysAddr::new(0x40E)
    // unsafe { & *(x86_64::PhysAddr::new(0x40E).as_u64() as *const u16) }
    // unsafe {core::ptr::read_volatile(0x40E as *const u16)}
    // crate::serial_println!("{} - {}",0x000E0000 as usize, 0x000FFFFF);
    // crate::serial_println!("{:?}", unsafe{& *(0x000FFFFF as *const u16)});
    
    #[cfg(test)]
    test_main(); // Useless, but compiler is angry without it.

    // print_all_pci_devices();
    // for device in kernel::pci::pci_device_iter() {
    //     if device.class == 1 {
    //         serial_println!("{:#b} - {:?} - {} - {}", device.prog_if, device.status, device.int_line, device.int_pin);
    //     }
    // }
    // kernel::boot::end()
    // Enter a 'sleep' phase (a.k.a. finished booting)
    hlt_loop()
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC:{}", info);
    kernel::hlt_loop();
}
