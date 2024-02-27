#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![feature(panic_info_message)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]
#![warn(clippy::all)]
#![deny(clippy::perf)]
#![deny(clippy::correctness)]

use core::fmt::Write;

use alloc::string::ToString;
use bootloader::{entry_point, BootInfo};
use kernel::{serial_print, serial_println, test::exit_qemu};

#[cfg(not(test))]
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    let executor = kernel::boot::boot(boot_info);
    log::info!("Done booting !");

    kernel::boot::end(executor)
}

extern crate alloc;
#[panic_handler]
#[track_caller]
fn panic(info: &core::panic::PanicInfo) -> ! {
    x86_64::instructions::interrupts::disable();
    use kernel::terminal::serial::SERIAL1;
    if SERIAL1.is_locked() {
        // In case panic occurs whilst printing something
        unsafe { SERIAL1.force_unlock() };
        serial_print!("\n"); // If we were printing there's a good chance the line wasn't finished
    }
    use alloc::string::ToString;
    let mut panic_msg = "\x1b[31;1m[PANIC]\x1b[0;0m: ".to_string();
    if let Some(message) = info.message() {
        panic_msg.push_str(&alloc::format!("{} -", message));
    } else {
        panic_msg.push_str("No message -");
    }
    if let Some(loc) = info.location() {
        serial_println!(
            "{} {}:{}:{}",
            panic_msg,
            loc.file(),
            loc.line(),
            loc.column()
        );
        let traces = kernel::log::TRACES.read();
        let firsts = traces.len().saturating_sub(10);
        serial_println!("\tTRACES: ");
        let mut traces_len = 0;
        for trace in traces[firsts..]
            .iter()
            .filter(|trace| return trace.contains(loc.file()))
        {
            serial_println!("[TRACE] {}", trace);
            traces_len += 1;
        }
        if traces_len == 0 {
            serial_println!("None for the specific module, printing without filter:");
            kernel::log::print_trace(10);
        }
    } else {
        serial_println!("{} No location", panic_msg);
        serial_println!("\tTRACES: ");
        kernel::log::print_trace(10);
    }
    if let Some(payload) = info.payload().downcast_ref::<&str>() {
        serial_println!("Panic payload: {:?}", payload);
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
