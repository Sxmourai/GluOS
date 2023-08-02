use alloc::{format, string::{String, ToString}, vec::Vec};
use spin::Mutex;

use crate::{serial_println, println, serial_print, dbg};
use lazy_static::lazy_static;

const ELAPSED_TICKS: Mutex<usize> = Mutex::new(0);
lazy_static!{static ref TRACE: Mutex<Vec<String>> = Mutex::new(Vec::new());}

pub fn tick() {
    *ELAPSED_TICKS.lock() += 1;
}
pub fn get_ticks() -> usize {*ELAPSED_TICKS.lock()}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 1,
    Error = 2,
    Info  = 3,
    Warn  = 4,
    Trace = 5,
}
pub fn print_trace() {
    let mut i = 0;
    let len = TRACE.lock().len();
    for trace in TRACE.lock().iter() {
        if (len-i) < 10 {
            serial_println!("- {}", trace);
        }
        i += 1;
    }
}

pub fn log(msg:impl core::fmt::Display, level: Level) {
    let color = match level {
        Level::Debug => "\x1b[1;37m[\x1b[1;32mDEBUG\x1b[1;37m] \x1b[;37m",
        Level::Error => "\x1b[1;37m[\x1b[1;31mERROR\x1b[1;37m] \x1b[;37m",
        Level::Info  => "\x1b[1;37m[\x1b[1;37mINFO\x1b[1;37m]  \x1b[;37m",
        Level::Warn  => "\x1b[1;37m[\x1b[1;33mWARN\x1b[1;37m]  \x1b[;37m",
        Level::Trace => "\x1b[1;37m[\x1b[1;37mTRACE\x1b[1;37m] \x1b[;37m",
    };
    let fmsg = format!("{}{}: {} \x1b[;37m", color, get_ticks(), msg);
    if level == Level::Trace {
        TRACE.lock().push(msg.to_string());
    } 
    else {
        serial_println!("{}", fmsg);
    }
}

#[macro_export]
macro_rules! log {
    ()                       => ($crate::log::log("Empty log", $crate::log::Level::Debug));
    ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Info));
    ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Info));
}

#[macro_export]
macro_rules! err {
    ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Error));
    ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Error));
}
#[macro_export]
macro_rules! warn {
    ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Warn));
    ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Warn));
}
#[macro_export]
macro_rules! trace {
    ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Trace));
    ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Trace));
}
#[macro_export]
macro_rules! dbg {
    ($fmt:expr)              => ($crate::log::log($fmt, $crate::log::Level::Debug));
    ($fmt:expr, $($arg:tt)*) => ($crate::log::log(alloc::format!($fmt, $($arg)*), $crate::log::Level::Debug));
}