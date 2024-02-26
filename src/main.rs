#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]

use core::fmt::Write;

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
    x86_64::instructions::interrupts::without_interrupts(|| {
        use kernel::terminal::serial::SERIAL1;
        if SERIAL1.is_locked() { // In case panic occurs whilst printing something
            unsafe {SERIAL1.force_unlock()};
            serial_print!("\n"); // If were printing there's a good chance the line wasn't finished
        }
        serial_println!("\x1b[31;1m[PANIC]\x1b[0;0m: {}", info);
        kernel::log::print_trace(10);
        kernel::test::end()
    })
}

#[cfg(test)]
entry_point!(test_kernel_main);
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::boot::boot(boot_info);
    test_main();
    kernel::test::end()
}
