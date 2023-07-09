#![no_std]
#![no_main]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]

extern crate kernel;
use kernel::{println, exit_qemu, QemuExitCode, serial_println};
use core::panic::PanicInfo;


#[test_case]
fn test_println() {
    println!("test_println output");
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    kernel::println!("Basic booting & vga printing tests !");
    test_main();

    kernel::end()
}


#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);
    loop {}
}