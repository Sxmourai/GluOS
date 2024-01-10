use alloc::string::String;
use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3F8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

use core::{fmt::{Write, Arguments}, str::FromStr};
use x86_64::instructions::interrupts;

use crate::dbg;
#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        SERIAL1.lock().write_fmt(args)
            .expect("Printing to serial failed");
    });
}

pub fn read_serial_input() -> u8 {
    use crate::x86_64::instructions::port::PortRead;
    interrupts::without_interrupts(|| {
        unsafe { u8::read_from_port(0x3F8) }
    })
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::terminal::serial::_print(format_args!($($arg)*))
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
