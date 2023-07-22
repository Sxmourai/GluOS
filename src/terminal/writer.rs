use alloc::vec::Vec;
use x86_64::structures::port::{PortWrite, PortRead};
use core::{fmt, arch::asm};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::serial_println;

use super::{console::{
    Console, ConsoleError, ScreenChar, DEFAULT_CHAR,
}, buffer::{Buffer, BUFFER_WIDTH, BUFFER_HEIGHT}};

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
pub struct ColorCode(pub u8);

impl ColorCode {
    pub const fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
    pub const fn newb(foreground: u8, background: u8) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct ScreenPos(pub usize, pub usize);

pub struct Writer {
    pub cursor_pos: ScreenPos,
    color_code: ColorCode,
    console: &'static Mutex<Console>,
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
                self.write_char_at(
                    self.cursor_pos.clone(),
                    ScreenChar {
                        ascii_character: byte,
                        color_code,
                    },
                );
                self.move_cursor(self.cursor_pos.0, self.cursor_pos.1+1);
            }
        }
    }
    fn move_cursor(&mut self, x: usize, y: usize) {
        self.cursor_pos = ScreenPos(x,y);

    }
    pub fn get_at(&self, pos: ScreenPos) -> ScreenChar {
        self.console.lock().get_atp(&pos)
    }
    pub fn write_char_at(&mut self, pos: ScreenPos, chr: ScreenChar) {
        self.console.lock().write_char_at(pos.0, pos.1, chr)
    }
    fn save_top_line(&mut self) { // Could use &self, but because we mutate console, I prefer explicitely making this mutable
        let mut console: spin::MutexGuard<'_, Console> = self.console.lock();
        let mut top_line = console.get_str_at(&ScreenPos(0, 0), console.size().1);
        drop(console); // Drop lock even if it reuse it afterwards
        let mut top_line_arr = [DEFAULT_CHAR; 80];
        for i in 0..80 {
            top_line_arr[i] = top_line.pop().unwrap(); // Can use unwrap because terminal width should always be 80
            //TODO make change 80 to the console.width attribute
        }
        drop(top_line);
        // Concats both buffer. Puts data at index 0, (pushes everything down)
        self.console.lock().top_buffer.append(top_line_arr);
    }
    pub fn move_up(&mut self) {
        let (width, height) = self.console.lock().size();
        x86_64::instructions::interrupts::without_interrupts(|| { // If press enter while executed, can do deadlocks ?
            //TODO Do we really need without_interrupts?
            // Move every line one up
            for row in 1..height {
                let line = self.console.lock().get_str_at(&ScreenPos(row, 0), width);
                self.write_screenchars_at(row-1, 0, line);
            }
            if !self.console.lock().bottom_buffer.is_empty() {
                self.write_screenchars_at(height, 0, self.console.lock().bottom_buffer.get_youngest_line().unwrap());
            }
        })
    }
    pub fn move_down(&mut self) {
        let (width, height) = self.console.lock().size();
        x86_64::instructions::interrupts::without_interrupts(|| { // If press enter while executed, can do deadlocks ?
            //TODO Do we really need without_interrupts?
            // Move every line one down
            // Iterate in reverse order because it would copy the same line every time
            // It's like write left to write as a left-handed, the ink would be go on the text you are currently writing (I know that, I'm left-handed) 
            for row in (0..height-1).rev() { // Same as -1 step in python
                let line: Vec<ScreenChar> = self.console.lock().get_str_at(&ScreenPos(row, 0), width);
                self.write_screenchars_at(row+1, 0, line);
            }
            if !self.console.lock().top_buffer.is_empty() {
                self.write_screenchars_at(0, 0, self.console.lock().top_buffer.get_youngest_line().unwrap());
            } else{
                self.write_screenchars_at(0, 0, [DEFAULT_CHAR; 80]);
            }
        })
    }
    pub fn write_screenchars_at(&mut self, mut row: usize, mut column: usize, s:impl IntoIterator<Item = ScreenChar>) {
        for screenchar in s.into_iter() {
            if column >= BUFFER_WIDTH {
                column = 0;
                if row + 1 == BUFFER_HEIGHT {
                    self.save_top_line();
                    self.move_down()
                } else {
                    row += 1
                }
            }
            self.write_char_at(
                ScreenPos(row, column),
                screenchar
            );
            column += 1;
        }
    }
    pub fn write_string_at(&mut self, mut row: usize, mut column: usize, s: &str) {
        self.write_screenchars_at(row, column, &mut s.split("").map(|chr| ScreenChar::from(chr.as_bytes()[0])))
    }
    pub fn write_string_atp(&mut self, pos: &ScreenPos, s: &str) {
        self.write_string_at(pos.0, pos.1, s)
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
        if self.cursor_pos.0 + 1 < BUFFER_HEIGHT {
            self.cursor_pos.0 += 1;
        } else {
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.get_at(ScreenPos(row, col));
                    self.write_char_at(ScreenPos(row - 1, col), character);
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

pub fn print_at(row: usize, column: usize, s: &str) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_at(row, column, s)
    })
}

pub fn print_char_at(row: usize, column: usize, c: char) {
    print_byte_at(row, column, c as u8)
}
pub fn print_byte_at(row: usize, column: usize, byte: u8) {
    print_screenchar_at(row, column, ScreenChar::from(byte))
}
pub fn print_screenchar_at(row: usize, column: usize, c: ScreenChar) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_char_at(ScreenPos(row, column), c);
    });
}
pub fn print_char_atp(pos: &ScreenPos, c: char) {
    print_byte_at(pos.0, pos.1, c as u8)
}
pub fn print_byte_atp(pos: &ScreenPos, byte: u8) {
    print_screenchar_at(pos.0, pos.1, ScreenChar::from(byte))
}
pub fn print_screenchar_atp(pos: &ScreenPos, c: ScreenChar) {
    print_screenchar_at(pos.0, pos.1, c)
}
pub fn print_atp(pos: &ScreenPos, s: &str) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_atp(pos, s);
    });
}
pub fn print_screenchars_atp(pos: &ScreenPos, s: impl IntoIterator<Item = ScreenChar>) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_screenchars_at(pos.0,pos.1, s);
    });
}

pub fn calculate_end(start: &ScreenPos, s: &str) -> ScreenPos {
    ScreenPos(
        start.0 + ((s.len() - start.1) / BUFFER_WIDTH),
        start.1 + (s.len() % BUFFER_WIDTH),
    )
}

pub unsafe fn outb(port:u16, data:u8) {
    // crate::serial_print!("Write 0b{:b} from port 0x{:x} - ", data, port);
    PortWrite::write_to_port(port, data);
    // asm!("out dx, al", in("al") data, in("dx") port);
    // serial_println!("Ok");
}

pub unsafe fn inb(port:u16) -> u8 {
    return PortRead::read_from_port(port);
    // let value: u32;
//     asm!("in eax, dx", out("eax") value, in("dx") port);
    // if (value != 0 && value.count_zeros() != 0) { // Check if it's not 0 // just ones
    //     serial_println!("Read 0b{:b} from port 0x{:x}", value, port);
    // }

    // value
}