#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]

use bootloader::{entry_point, BootInfo};
use kernel::{serial_print, serial_println, test::exit_qemu};

#[cfg(not(test))]
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    let executor = kernel::boot::boot(boot_info);
    log::info!("Done booting !");

    kernel::boot::end(executor)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("\x1b[31;1m[PANIC]\x1b[0;0m: {}", info);
    // Prints traceback of 10 last items
    let traces = kernel::user::log::TRACES.read();
    let firsts = traces.len().saturating_sub(10);
    for trace in traces[firsts..].iter() {
        serial_println!("[TRACE] {}", trace);
    }
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
