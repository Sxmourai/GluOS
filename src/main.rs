#![no_std]
#![no_main]
#![feature(custom_test_frameworks)] // Required for ´cargo test´ because it searches in main.rs even if no tests
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_main"]


extern crate kernel;
extern crate bootloader;
extern crate x86_64;
extern crate alloc;

use core::panic::PanicInfo;
use alloc::string::ToString;
use kernel::{serial_println, prompt::Prompt};
use bootloader::{BootInfo, entry_point};

entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::init(boot_info);
    #[cfg(test)]
    test_main();
    kernel::prompt::BlockingPrompt::new(">".to_string()).run();
    kernel::end();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC:{}", info);
    kernel::hlt_loop();
}
