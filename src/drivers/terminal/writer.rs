use alloc::vec::Vec;
use core::fmt;
use lazy_static::lazy_static;

use spin::Mutex;
use x86_64::structures::port::PortWrite;

use crate::{serial_println, terminal::buffer::VgaBuffer};

use super::{
    buffer::{BUFFER_HEIGHT, BUFFER_WIDTH, SBUFFER_WIDTH},
    console::{Console, ScreenChar, DEFAULT_CHAR},
};

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
    Bxn = 6,
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
        return ColorCode((background as u8) << 4 | (foreground as u8))
    }
    pub const fn newb(foreground: u8, background: u8) -> ColorCode {
        return ColorCode(background << 4 | foreground)
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct ScreenPos(pub u8, pub u8);

pub struct Writer {
    pub pos: ScreenPos,
    color_code: ColorCode,
    console: Console,
}

impl Writer {
    // Function to move the cursor to a specific position in the VGA buffer
    pub fn move_cursor(&mut self, x: u8, y: u8) {
        self.pos = ScreenPos(x, y);

        let pos: u16 = y as u16 * 80 + x as u16;

        if self
            .console
            .get_char_at(self.pos.0, self.pos.1)
            .ascii_character
            == DEFAULT_CHAR.ascii_character
        {
            self.write_char_at(
                self.pos.clone(),
                ScreenChar::new(b'\0', ColorCode::new(Color::White, Color::Black)),
            );
        }
        unsafe {
            PortWrite::write_to_port(0x3D4, 0x0F_u8);
            PortWrite::write_to_port(0x3D5, pos);
            PortWrite::write_to_port(0x3D4, 0x0E_u8);
            PortWrite::write_to_port(0x3D5, (pos >> 8));
        }
    }
    pub fn write_char_at(&mut self, pos: ScreenPos, chr: ScreenChar) {
        self.console.write_char_at(pos.0, pos.1, chr)
    }
    /// Move every line one up
    pub fn move_up(&mut self) {
        let (width, height) = self.console.size();
        x86_64::instructions::interrupts::without_interrupts(|| {
            let mut schrs = [ScreenChar::default(); SBUFFER_WIDTH];
            for (i, c) in self
                .console
                .get_str_at(&ScreenPos(0, 0), width as u16)
                .iter()
                .enumerate()
            {
                schrs[i] = *c
            }
            self.console.top_buffer.append(schrs);
            for y in 1..height {
                let line = self.console.get_str_at(&ScreenPos(0, y), width as u16);
                self.write_screenchars_at_no_wrap(0, y - 1, line.iter());
            }
            if self.console.bottom_buffer.is_empty() {
                self.write_screenchars_at_no_wrap(0, height - 1, [DEFAULT_CHAR; 80].iter());
                if self.pos.1 != 0 {
                    self.move_cursor(self.pos.0, self.pos.1 - 1)
                } else {
                    //TODO Fix errors here
                    serial_println!("Bug whilst moving cursor: {:?}", self.pos);
                }
            } else {
                self.write_screenchars_at_no_wrap(
                    0,
                    height - 1,
                    self.console
                        .bottom_buffer
                        .get_youngest_line()
                        .unwrap()
                        .iter(),
                );
                self.console.bottom_buffer.remove_youngest_line();
            }
        })
    }
    /// Move every line one down
    pub fn move_down(&mut self) {
        let (width, height) = self.console.size();
        x86_64::instructions::interrupts::without_interrupts(|| {
            // Move every line one down
            // Iterate in reverse order because it would copy the same line every time
            // It's like write left to write as a left-handed, the ink would be go on the text you are currently writing (I know that, I'm left-handed)

            let mut schrs = [ScreenChar::default(); SBUFFER_WIDTH];
            for (i, c) in self
                .console
                .get_str_at(&ScreenPos(0, height - 1), width as u16)
                .iter()
                .enumerate()
            {
                schrs[i] = *c
            }
            self.console.bottom_buffer.append(schrs);
            for y in 2..=height {
                let line = self
                    .console
                    .get_str_at(&ScreenPos(0, height - y), width as u16);
                self.write_screenchars_at_no_wrap(0, height - y + 1, line.iter());
            }
            if self.console.top_buffer.is_empty() {
                self.write_screenchars_at_no_wrap(
                    0,
                    0,
                    [ScreenChar::default(); SBUFFER_WIDTH].iter(),
                );
                self.move_cursor(self.pos.0, self.pos.1 + 1)
            } else {
                self.write_screenchars_at_no_wrap(
                    0,
                    0,
                    self.console.top_buffer.get_youngest_line().unwrap().iter(),
                );
                self.console.top_buffer.remove_youngest_line();
            }
        })
    }
    pub fn write_screenchars_at_no_wrap<'a>(
        &mut self,
        x: u8,
        y: u8,
        s: impl Iterator<Item = &'a ScreenChar>,
    ) {
        for (i, char) in s.enumerate() {
            self.write_char_at(ScreenPos(x + i as u8, y), *char);
        }
    }
    pub fn write_screenchars_at(
        &mut self,
        mut x: u8,
        mut y: u8,
        s: impl IntoIterator<Item = ScreenChar>,
    ) -> (u8, u8) {
        for c in s.into_iter() {
            if (x + 1 >= BUFFER_WIDTH) || (c.ascii_character == b'\n') {
                if y + 1 >= BUFFER_HEIGHT {
                    self.move_up()
                } else {
                    y += 1
                }
                x = 0;
            }
            if c.ascii_character == b'\n' {
                continue;
            }
            self.write_char_at(ScreenPos(x, y), c);
            x += 1;
        }
        return (x, y)
    }
    // Prints characters at desired position, with color of self.color_code and returns the end index
    pub fn write_string_at(&mut self, x: u8, y: u8, s: &str) -> (u8, u8) {
        let mut screenchars = Vec::new();
        for c in s.bytes() {
            screenchars.push(ScreenChar {
                ascii_character: c,
                color_code: self.color_code,
            })
        }

        return self.write_screenchars_at(x, y, screenchars)
    }
    pub fn write_string_atp(&mut self, pos: &ScreenPos, s: &str) -> (u8, u8) {
        return self.write_string_at(pos.0, pos.1, s)
    }

    pub fn write_string(&mut self, s: &str) {
        let (x, y) = self.write_string_atp(&self.pos.clone(), s);
        self.move_cursor(x, y);
    }

    pub fn new_line(&mut self) {
        if self.pos.1 + 1 < BUFFER_HEIGHT {
            self.move_cursor(0, self.pos.1 + 1)
        } else {
            // Move everything
            self.move_up();
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        return Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        pos: ScreenPos(0, 0),
        color_code: ColorCode::new(Color::White, Color::Black),
        console: Console::new(unsafe { &mut *(0xb8000 as *mut VgaBuffer) })
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::terminal::writer::_print(format_args!($($arg)*)))
}
// #[macro_export]
// macro_rules! print_at {
//     ($x, $y, $($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
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
    })
}

pub fn print_at(x: u8, y: u8, s: &str) -> (u8, u8) {
    return x86_64::instructions::interrupts::without_interrupts(|| return WRITER.lock().write_string_at(x, y, s))
}

pub fn print_char_at(x: u8, y: u8, c: char) {
    print_byte_at(x, y, c as u8)
}
pub fn print_byte_at(x: u8, y: u8, byte: u8) {
    print_screenchar_at(x, y, ScreenChar::from(byte))
}
pub fn print_screenchar_at(x: u8, y: u8, c: ScreenChar) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_char_at(ScreenPos(x, y), c);
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
        WRITER.lock().write_screenchars_at(pos.0, pos.1, s);
    });
}

pub fn calculate_end(start: &ScreenPos, len: usize) -> ScreenPos {
    return ScreenPos(
        start.0 + (len % SBUFFER_WIDTH) as u8,
        start.1 + ((len + start.0 as usize) / SBUFFER_WIDTH) as u8,
    )
}
