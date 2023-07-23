use alloc::vec::Vec;
use x86_64::structures::port::{PortWrite, PortRead};
use core::{fmt, arch::asm};
use lazy_static::lazy_static;
use spin::Mutex;

use crate::serial_println;

use super::{console::{
    Console, ConsoleError, ScreenChar, DEFAULT_CHAR,
}, buffer::{Buffer, BUFFER_WIDTH, BUFFER_HEIGHT, SBUFFER_WIDTH}};

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
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
    pub const fn newb(foreground: u8, background: u8) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Hash)]
pub struct ScreenPos(pub u8, pub u8);

pub struct Writer {
    pub pos: ScreenPos,
    color_code: ColorCode,
    console: &'static Mutex<Console>,
}

impl Writer {
    // Function to move the cursor to a specific position in the VGA buffer
    pub fn move_cursor(&mut self, x: u8, y: u8) {
        self.pos = ScreenPos(x,y);
        let pos: u16 = (y as u16 * 80 + x as u16);
        
        if self.get_at(&self.pos).ascii_character == DEFAULT_CHAR.ascii_character {
            self.write_char_at(self.pos.clone(), ScreenChar::new('\0' as u8, ColorCode::new(Color::White, Color::Black)));
        }
        unsafe {
            outb(0x3D4, 0x0F);
            outb16(0x3D5, (pos).try_into().unwrap());
            outb(0x3D4, 0x0E);
            outb16(0x3D5, (pos >> 8).try_into().unwrap());
        }
    }
    pub fn get_at(&self, pos: &ScreenPos) -> ScreenChar {
        self.console.lock().get_atp(pos)
    }
    pub fn write_char_at(&mut self, pos: ScreenPos, chr: ScreenChar) {
        self.console.lock().write_char_at(pos.0, pos.1, chr)
    }
    fn save_top_line(&mut self) { // Could use &self, but because we mutate console, I prefer explicitely making this mutable
        let mut console: spin::MutexGuard<'_, Console> = self.console.lock();
        let mut top_line = console.get_str_at(&ScreenPos(0, 0), console.size().1 as u16);
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
            for y in 1..height {
                let line = self.console.lock().get_str_at(&ScreenPos(0, y), width as u16);
                self.write_screenchars_at(0, y-1, line);
            }
            if !self.console.lock().bottom_buffer.is_empty() {
                self.write_screenchars_at(0, height, self.console.lock().bottom_buffer.get_youngest_line().unwrap());
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
            for x in (0..height-1).rev() { // Same as -1 step in python
                let line: Vec<ScreenChar> = self.console.lock().get_str_at(&ScreenPos(x, 0), width as u16);
                self.write_screenchars_at(x+1, 0, line);
            }
            if !self.console.lock().top_buffer.is_empty() {
                self.write_screenchars_at(0, 0, self.console.lock().top_buffer.get_youngest_line().unwrap());
            } else{
                self.write_screenchars_at(0, 0, [DEFAULT_CHAR; 80]);
            }
        })
    }
    pub fn write_screenchars_at(&mut self, mut x: u8, mut y: u8, s:impl IntoIterator<Item = ScreenChar>) {
        for screenchar in s.into_iter() {
            if x >= BUFFER_WIDTH {
                x = 0;
                if y + 1 == BUFFER_HEIGHT {
                    self.save_top_line();
                    self.move_down()
                } else {
                    y += 1
                }
            }
            self.write_char_at(
                ScreenPos(x, y),
                screenchar
            );
            x += 1;
        }
    }
    // Prints characters at desired position, with color of self.color_code and returns the end index
    pub fn write_string_at(&mut self, mut x: u8, mut y: u8, s: &str) -> (u8,u8) {
        for byte in s.bytes() {
            match byte {
                b'\n' => {x = 0; y+=1},
                byte => {
                    if x > BUFFER_WIDTH {
                        x = 0;
                        if y+1 < BUFFER_HEIGHT {
                            y += 1
                        } else {
                            self.move_up()
                        }
                    }
                    self.write_char_at(
                        ScreenPos(x, y),
                        ScreenChar {
                            ascii_character: byte,
                            color_code: self.color_code,
                        },
                    );
                    x += 1;
                }
            }
        }
        (x,y)
    }
    pub fn write_string_atp(&mut self, pos: &ScreenPos, s: &str) -> (u8,u8) {
        self.write_string_at(pos.0, pos.1, s)
    }

    pub fn write_string(&mut self, s: &str) {
        let (x,y) = self.write_string_atp(&self.pos.clone(), s);
        self.move_cursor(x,y);
    }

    pub fn new_line(&mut self) {
        if self.pos.1 + 1 < BUFFER_HEIGHT {
            self.move_cursor(0, self.pos.1+1)
        } else { // Move everything
            for y in 1..BUFFER_HEIGHT {
                for x in 0..BUFFER_WIDTH {
                    let character = self.get_at(&ScreenPos(x, y));
                    self.write_char_at(ScreenPos(x, y-1), character);
                }
            }
            for x in 0..BUFFER_WIDTH {
                self.write_char_at(ScreenPos(x, BUFFER_HEIGHT - 1), DEFAULT_CHAR);
            }
        }
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
        pos: ScreenPos(0, 0),
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
    });
}

pub fn print_at(x: u8, y: u8, s: &str) -> (u8,u8) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        WRITER.lock().write_string_at(x, y, s)
    })
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
        WRITER.lock().write_screenchars_at(pos.0,pos.1, s);
    });
}

pub fn calculate_end(start: &ScreenPos, len:usize) -> ScreenPos {
    ScreenPos(
        start.0 + (len % SBUFFER_WIDTH) as u8,
        start.1 + ((len + start.0 as usize) / SBUFFER_WIDTH) as u8,
    )
}



pub unsafe fn outb(port:u16, data:u8) {
    // crate::serial_print!("Write 0b{:b} from port 0x{:x} - ", data, port);
    PortWrite::write_to_port(port, data)
    // asm!("out dx, al", in("al") data, in("dx") port);
    // serial_println!("Ok");
}
pub unsafe fn outb16(port:u16, data:u16) {
    PortWrite::write_to_port(port, data)
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