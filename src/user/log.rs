use alloc::{format, string::{String, ToString}, vec::Vec};
use spin::Mutex;

use crate::{serial_println, println, serial_print};
use lazy_static::lazy_static;
use log::{Level, LevelFilter, Log, Metadata, Record, error};

//TODO Have all codes https://chrisyeh96.github.io/2020/03/28/terminal-colors.html
enum Color {
    Black = 0,
    Red = 1,
    Green = 2,
    Yellow = 3,
    Blue = 4,
    Magenta = 5,
    Cyan = 6,
    White = 7,
}
impl From<u8> for Color {
    fn from(value: u8) -> Self {
        match value {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            _ => {error!("From u8 but u8 is too big !");Color::White}
        }
    }
}
enum Codes {
    Reset = 0,
    Bold = 1,
    Dim = 2,
    Underline = 4,
    SlowBlink = 5,
}
impl Codes {
    fn reset() -> &'static str {"\x1b[0;0m"}
}

struct Logger;
impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // Implement your own logic to determine if the log level is enabled
        metadata.level() <= MAX_LEVEL
    }
    fn log(&self, record: &Record) {
        // Implement your own logic to handle the log record
        let mut buffer = [0u8; 128];
        let color = format!("\x1b[1;3{}m", Color::from(record.level() as u8) as u8);
        serial_println!("[{}{}{}] {}", color, record.level(),Codes::reset(), record.args());
        // Your logic to write to an output (e.g., a memory buffer)
    }
    //TODO
    fn flush(&self) {
        todo!("Flush log");
        // Implement your own logic to flush the log messages
    }
}
const MAX_LEVEL: Level = Level::Trace;
pub fn initialize_logger() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Debug);
}












// const ELAPSED_TICKS: Mutex<usize> = Mutex::new(0);
// lazy_static!{static ref TRACE: Mutex<Vec<String>> = Mutex::new(Vec::new());}

// pub fn tick() {
//     *ELAPSED_TICKS.lock() += 1;
// }
// pub fn get_ticks() -> usize {*ELAPSED_TICKS.lock()}

// #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
// pub enum LogLevel {
//     Debug = 1,
//     Error = 2,
//     Info  = 3,
//     Warn  = 4,
//     Trace = 5,
// }
// pub fn print_trace() {
//     let mut i = 0;
//     let len = TRACE.lock().len();
//     for trace in TRACE.lock().iter() {
//         if (len-i) < 10 {
//             serial_println!("- {}", trace);
//         }
//         i += 1;
//     }
// }

// pub fn log(msg:impl core::fmt::Display, level: Level) {
//     let color = match level {
//         Level::Debug => "\x1b[1;37m[\x1b[1;32mDEBUG\x1b[1;37m] \x1b[;37m",
//         Level::Error => "\x1b[1;37m[\x1b[1;31mERROR\x1b[1;37m] \x1b[;37m",
//         Level::Info  => "\x1b[1;37m[\x1b[1;37mINFO\x1b[1;37m]  \x1b[;37m",
//         Level::Warn  => "\x1b[1;37m[\x1b[1;33mWARN\x1b[1;37m]  \x1b[;37m",
//         Level::Trace => "\x1b[1;37m[\x1b[1;37mTRACE\x1b[1;37m] \x1b[;37m",
//     };
//     let fmsg = format!("{}{}: {} \x1b[;37m", color, get_ticks(), msg);
//     if level == Level::Trace {
//         TRACE.lock().push(msg.to_string());
//         #[cfg(feature = "print_trace")]
//         serial_println!("{}", fmsg);
//     } 
//     else {
//         serial_println!("{}", fmsg);
//     }
// }

// #[macro_export]
// macro_rules! log {
//     ()                       => ($crate::log::log("Empty log", $crate::log::Level::Debug));
//     ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Info));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Info));
// }

// #[macro_export]
// macro_rules! err {
//     ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Error));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Error));
// }
// #[macro_export]
// macro_rules! warn {
//     ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Warn));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Warn));
// }
// #[macro_export]
// macro_rules! trace {
//     ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Trace));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Trace));
// }
// #[macro_export]
// macro_rules! dbg {
//     ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Debug));
//     ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Debug));
// }

pub fn point() {
    log::debug!("Code went all the way there !");
}