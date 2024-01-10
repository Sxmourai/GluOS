#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;
extern crate bootloader;
extern crate kernel;
extern crate log;
extern crate x86_64;
use bootloader::{entry_point, BootInfo};
use kernel::serial_print;
use core::
    panic::PanicInfo
;
use log::{error, info};

// const CONFIG: bootloader_api::BootloaderConfig = {
//     let mut config = bootloader_api::BootloaderConfig::new_default();
//     config.kernel_stack_size = 100 * 1024; // 100 KiB
//     config
// };

#[cfg(not(test))]
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::boot::boot(boot_info);

    info!("Done booting !");

    kernel::boot::end()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_print!("PANIC");
    error!("PANIC: {}", info);
    kernel::test::end()
}

#[cfg(test)]
entry_point!(test_kernel_main);
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::boot::boot(boot_info);
    test_main();
    kernel::test::end()
}
