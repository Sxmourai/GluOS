use alloc::format;

use crate::serial_println;

use log::{error, Level, LevelFilter, Log, Metadata, Record};

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
        // Implement your own logic to determine if the log level is enabled
        metadata.level() <= MAX_LEVEL
    }
    fn log(&self, record: &Record) {
        // Implement your own logic to handle the log record
        let _buffer = [0u8; 128];
        let color = format!("\x1b[1;3{}m", Color::from(record.level() as u8) as u8);
        serial_println!(
            "[{}{}{}] {}",
            color,
            record.level(),
            Codes::reset(),
            record.args()
        );
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

pub fn point() {
    log::debug!("Code went all the way there !");
}

#[macro_export]
macro_rules! dbg {
    ($variable:expr) => {
        serial_println!(
            "{} = {:?} at {}:{}",
            stringify!($variable),
            $variable,
            file!(),
            line!(),
        )
    };
}
