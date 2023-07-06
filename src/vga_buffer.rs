use alloc::vec::Vec;
use volatile::Volatile;
use lazy_static::lazy_static;
use core::fmt;
use spin::Mutex;

use crate::serial_println;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;


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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: ColorCode,
}
impl ScreenChar {
    pub const fn new(ascii_character: u8, color_code: ColorCode) -> ScreenChar {
        ScreenChar { ascii_character, color_code }
    }
    pub const fn from(ascii_character: u8) -> ScreenChar {
        ScreenChar { ascii_character, color_code: ColorCode::new(Color::White, Color::Black) }
    }
}

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct ScreenPos(pub usize, pub usize);
#[derive(Debug)]
pub enum VGAError {
    OutOfBounds
}

static INVIS_CHAR:ScreenChar = ScreenChar::from(0x00);
#[repr(transparent)]
#[derive(Debug)]
pub struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT], // [row][column]
}
impl Buffer {
    pub fn remove(&mut self, row:usize, column:usize) {
        self.chars[row][column].write(INVIS_CHAR);
    }
}
pub struct Writer {
    pub cursor_pos: ScreenPos,
    color_code: ColorCode,
    pub buffer: &'static mut Buffer,
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
                self.write_char_at(self.cursor_pos.0, self.cursor_pos.1, &ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.cursor_pos.1 += 1;
            }
        }
    }
    pub fn write_string_at(&mut self, mut row:usize,mut column:usize, s: &str) -> Result<(), VGAError> {
        for byte in s.bytes() {
            if column >= BUFFER_WIDTH {
                column = 0;
                if row+1 == BUFFER_HEIGHT {return Result::Err(VGAError::OutOfBounds)}
                else {row += 1}
            }
            self.write_byte_at(row,column, byte);
            column += 1;
        }
        Ok(())
    }
    pub fn write_string_atp(&mut self, pos:&ScreenPos, s:&Vec<ScreenChar>) -> Result<(), VGAError> {
        let (mut row, mut column) = (pos.0, pos.1);
        for chr in s {
            if column >= BUFFER_WIDTH {
                column = 0;
                if row+1 == BUFFER_HEIGHT {return Result::Err(VGAError::OutOfBounds)}
                else {row += 1}
            }
            self.write_char_at(row, column, chr);
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
                    let character = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(character);
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
        }
        self.cursor_pos.1 = 0;
    }
    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
    pub fn clear(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            self.clear_row(row);
        }
    }
    pub fn write_char_at(&mut self, row:usize, column:usize, character:&ScreenChar) {
        // serial_println!("Printing {} at r:{} c:{}", character.ascii_character as char, row, column);
        self.buffer.chars[row][column].write(*character)
    }
    pub fn write_byte_at(&mut self, row:usize, column:usize, byte:u8) {
        self.write_char_at(row, column, &ScreenChar::from(byte))
    }
    pub fn get_at(&mut self, row:usize, column:usize) -> ScreenChar {
        self.buffer.chars[row][column].read()
    }
    pub fn get_atp(&mut self, pos:ScreenPos) -> ScreenChar {
        self.get_at(pos.0, pos.1)
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
        color_code: ColorCode::new(Color::White, Color::Green),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}
pub fn clear_screen() {
    WRITER.lock().clear()
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
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

pub fn print_at(row:usize, column:usize, s:&str) -> Result<(), VGAError>{
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_at(row, column, s)
    })
}

pub fn print_char_at(row:usize, column:usize, c:char) {
    print_byte_at(row, column, c as u8)
}
pub fn print_byte_at(row:usize, column:usize, byte:u8) {
    print_screenchar_at(row, column, &ScreenChar::from(byte))
}
pub fn print_screenchar_at(row:usize, column:usize, c:&ScreenChar) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_char_at(row, column, c);
    });
}
pub fn print_char_atp(pos:&ScreenPos, c:char) {
    print_byte_at(pos.0, pos.1, c as u8)
}
pub fn print_byte_atp(pos:&ScreenPos, byte:u8) {
    print_screenchar_at(pos.0, pos.1, &ScreenChar::from(byte))
}
pub fn print_screenchar_atp(pos:&ScreenPos, c:&ScreenChar) {
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