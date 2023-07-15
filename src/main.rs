#![no_std]
#![no_main]
#![feature(custom_test_frameworks)] // Required for ´cargo test´ because it searches in main.rs even if no tests
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]
#![allow(unused)]

extern crate kernel;
extern crate bootloader;
extern crate x86_64;
extern crate alloc;

use core::panic::PanicInfo;
use crate::kernel::{serial_println, hlt_loop};
use alloc::vec::Vec;
use bootloader::{BootInfo, entry_point};
use kernel::{serial_print, println};
use pci_ids::SubSystem;
use x86_64::VirtAddr;


entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::init(boot_info);



    // let to_print = memory::search_for_on_bios(&mut kernel::state::get_mem_handler().get_mut().frame_allocator);
    // crate::serial_println!("{:?}", to_print);

    // x86_64::PhysAddr::new(0x40E)
    // unsafe { & *(x86_64::PhysAddr::new(0x40E).as_u64() as *const u16) }
    // unsafe {core::ptr::read_volatile(0x40E as *const u16)}
    // crate::serial_println!("{} - {}",0x000E0000 as usize, 0x000FFFFF);
    // crate::serial_println!("{:?}", unsafe{& *(0x000FFFFF as *const u16)});
    
    #[cfg(test)]
    test_main();

    // print_all_pci_devices();
    // for device in kernel::pci::pci_device_iter() {
    //     if device.class == 1 {
    //         serial_println!("{:#b} - {:?} - {} - {}", device.prog_if, device.status, device.int_line, device.int_pin);
    //     }
    // }

    hlt_loop()
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC:{}", info);
    kernel::hlt_loop();
}
