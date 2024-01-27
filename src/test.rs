use crate::{boot::hlt_loop, serial_print, serial_println};
use core::panic::PanicInfo;
use x86_64::instructions::port::Port;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}
pub fn end() -> ! {
    exit_qemu(QemuExitCode::Success);
    hlt_loop()
}
pub fn runner(tests: &[&dyn Testable]) -> ! {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    end()
}
pub fn panic_handler(info: &PanicInfo) -> ! {
    serial_println!("[failed]\n");
    serial_println!("Error: {}\n", info);
    // exit_qemu(QemuExitCode::Failed);
    hlt_loop()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}
