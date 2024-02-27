use alloc::{format, string::String};

use crate::{serial_println, sync::TimeOutRwLock};

use log::{error, Level, LevelFilter, Log, Metadata, Record};

//TODO use a terminal library for colors and more ?
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
            0 => return Color::Black,
            1 => return Color::Red,
            2 => return Color::Green,
            3 => return Color::Yellow,
            4 => return Color::Blue,
            5 => return Color::Magenta,
            6 => return Color::Cyan,
            7 => return Color::White,
            _ => {
                error!("From u8 but u8 is too big !");
                return Color::White
            }
        }
    }
}
pub enum Codes {
    Reset = 0,
    Bold = 1,
    Dim = 2,
    Underline = 4,
    SlowBlink = 5,
}
impl Codes {
    fn reset() -> &'static str {
        return "\x1b[0;0m"
    }
}

struct Logger;
impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        // metadata.level() <= MAX_LEVEL
        return true
    }
    #[track_caller]
    fn log(&self, record: &Record) {
        let _buffer = [0_u8; 128];
        let args = match record.level() {
            Level::Trace => {
                let args = alloc::format!("{}:{}\t - {}", file!(), line!(), record.args());
                crate::user::log::TRACES.write_with_timeout().push(args.clone());
                return; // We don't want to print traces
            }
            _ => {
                alloc::format!("{}", record.args())
            }
        };
        let msg = format!(
            "[\x1b[1;3{}m{}{}] {}",
            Color::from(record.level() as u8) as u8,
            record.level(),
            Codes::reset(),
            record.args()
        );
        #[cfg(debug_assertions)]
        crate::serial_println!("{}", msg);
        #[cfg(not(debug_assertions))]
        crate::println!("{}", msg);
    }
    fn flush(&self) {
        todo!("Flush log");
    }
}
const MAX_LEVEL: Level = Level::Trace;
/// Initialises a log system for the os (sends logs to qemu if in debug mode)
pub fn init() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Trace);
}

#[macro_export]
macro_rules! dbg {
    () => {
        $crate::dbg!("Nothing")
    };
    ($variable:expr) => {
        $crate::serial_println!(
            "{} = {:?} at {}:{}",
            stringify!($variable),
            $variable,
            file!(),
            line!(),
        )
    };
    ($($var:expr),+ $(,)?) => {
        $(
            $crate::serial_print!(
                "{} = {:?}, ",
                stringify!($var),
                $var,
            );
        )+
        $crate::serial_println!(
            "at {}:{}",
            file!(),
            line!(),
        );
    };
}

#[macro_export]
macro_rules! pretty_dbg {
    ($variable:expr) => {
        $crate::serial_println!(
            "{} = {:#?} at {}:{}",
            stringify!($variable),
            $variable,
            file!(),
            line!(),
        )
    };
    ($($var:expr),+ $(,)?) => {
        $(
            $crate::serial_print!(
                "{} = {:#?}, ",
                stringify!($var),
                $var,
            );
        )+
        $crate::serial_println!(
            "at {}:{}",
            file!(),
            line!(),
        );
    };
}
use alloc::vec::Vec;
use spin::RwLock;
pub static TRACES: RwLock<Vec<String>> = RwLock::new(Vec::new());

// Prints traceback of N last items
pub fn print_trace(n: usize) {
    let traces = TRACES.read();
    let firsts = traces.len().saturating_sub(n);
    for trace in traces[firsts..].iter() {
        serial_println!("[TRACE] {}", trace);
    }
}
