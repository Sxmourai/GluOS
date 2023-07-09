use alloc::vec::Vec;
use lazy_static::lazy_static;
use core::fmt;
use spin::Mutex;

use super::console::{Console, BUFFER_WIDTH, ScreenChar, BUFFER_HEIGHT, ConsoleError, DEFAULT_CHAR};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct ScreenPos(pub usize,pub usize);


pub struct Writer {
    pub cursor_pos: ScreenPos,
    color_code: ColorCode,
    console:&'static Mutex<Console>
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.cursor_pos.1 >= BUFFER_WIDTH {
                    self.new_line();
                }

                let color_code = self.color_code;
                self.write_char_at(self.cursor_pos.clone(), ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.cursor_pos.1 += 1;
            }
        }
    }
    pub fn get_at(&self, pos:ScreenPos) -> ScreenChar {self.console.lock().get_atp(&pos)}
    pub fn write_char_at(&mut self, pos:ScreenPos, chr:ScreenChar) {self.console.lock().write_char_at(pos.0, pos.1, chr)}
    pub fn write_string_at(&mut self, mut row:usize,mut column:usize, s: &str) -> Result<(), ConsoleError> {
        for byte in s.bytes() {
            if column >= BUFFER_WIDTH {
                column = 0;
                if row+1 == BUFFER_HEIGHT {return Result::Err(ConsoleError::OutOfBounds)}
                else {row += 1}
            }
            self.write_char_at(ScreenPos(row,column), ScreenChar{ascii_character: byte, color_code: self.color_code});
            column += 1;
        }
        Ok(())
    }
    pub fn write_string_atp(&mut self, pos:&ScreenPos, s:&Vec<ScreenChar>) -> Result<(), ConsoleError> {
        let (mut row, mut column) = (pos.0, pos.1);
        for chr in s {
            if column >= BUFFER_WIDTH {
                column = 0;
                if row+1 == BUFFER_HEIGHT {return Result::Err(ConsoleError::OutOfBounds)}
                else {row += 1}
            }
            self.write_char_at(ScreenPos(row, column), *chr);
            column += 1;
        }
        Ok(())
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // not part of printable ASCII range
                _ => self.write_byte(0xfe),
            }

        }
    }

    pub fn new_line(&mut self) {
        if self.cursor_pos.0+1 < BUFFER_HEIGHT {self.cursor_pos.0 += 1;}
        else {
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.get_at(ScreenPos(row, col));
                    self.write_char_at(ScreenPos(row -1, col), character);
                }
            }
            for col in 0..BUFFER_WIDTH {
                self.write_char_at(ScreenPos(BUFFER_HEIGHT - 1, col), DEFAULT_CHAR);
            }
        }
        self.cursor_pos.1 = 0;
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        cursor_pos: ScreenPos(0, 0),
        color_code: ColorCode::new(Color::White, Color::Black),
        console: &crate::terminal::console::CONSOLE
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::writer::_print(format_args!($($arg)*)));
}
// #[macro_export]
// macro_rules! print_at {
//     ($row, $column, $($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
// }


#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).unwrap();
    });
}

pub fn print_at(row:usize, column:usize, s:&str) -> Result<(), ConsoleError>{
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_at(row, column, s)
    })
}

pub fn print_char_at(row:usize, column:usize, c:char) {
    print_byte_at(row, column, c as u8)
}
pub fn print_byte_at(row:usize, column:usize, byte:u8) {
    print_screenchar_at(row, column, ScreenChar::from(byte))
}
pub fn print_screenchar_at(row:usize, column:usize, c:ScreenChar) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_char_at(ScreenPos(row, column), c);
    });
}
pub fn print_char_atp(pos:&ScreenPos, c:char) {
    print_byte_at(pos.0, pos.1, c as u8)
}
pub fn print_byte_atp(pos:&ScreenPos, byte:u8) {
    print_screenchar_at(pos.0, pos.1, ScreenChar::from(byte))
}
pub fn print_screenchar_atp(pos:&ScreenPos, c:ScreenChar) {
    print_screenchar_at(pos.0, pos.1, c)
}
pub fn print_atp(pos:&ScreenPos, s:&Vec<ScreenChar>) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_atp(pos, s).unwrap();
    });
}



pub fn calculate_end(start:&ScreenPos, s:&str) -> ScreenPos {
    ScreenPos(start.0+((s.len()-start.1)/BUFFER_WIDTH), start.1+(s.len()%BUFFER_WIDTH))
}