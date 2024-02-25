use alloc::{format, string::String};

use crate::serial_println;

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
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            _ => {
                error!("From u8 but u8 is too big !");
                Color::White
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
        "\x1b[0;0m"
    }
}

struct Logger;
impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= MAX_LEVEL
    }
    fn log(&self, record: &Record) {
        let _buffer = [0u8; 128];
        #[cfg(debug_assertions)]
        serial_println!(
            "[\x1b[1;3{}m{}{}] {}",
            Color::from(record.level() as u8) as u8,
            record.level(),
            Codes::reset(),
            record.args()
        );
        #[cfg(not(debug_assertions))]
        crate::println!(
            "[\x1b[1;3{}m{}{}] {}",
            Color::from(record.level() as u8) as u8,
            record.level(),
            Codes::reset(),
            record.args()
        );
    }
    fn flush(&self) {
        todo!("Flush log");
    }
}
const MAX_LEVEL: Level = Level::Trace;
/// Initialises a log system for the os (sends logs to qemu if in debug mode)
pub fn initialize_logger() {
    static LOGGER: Logger = Logger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(LevelFilter::Debug);
}

#[macro_export]
macro_rules! dbg {
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

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        let args = alloc::format!("{}:{}\t - {}",file!(),line!(), alloc::format!($($arg)*));
        crate::user::log::TRACES.write().push(args.clone());
        log::trace!("{}", args)
    };
}
