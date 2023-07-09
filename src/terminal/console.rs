use spin::Mutex;
use volatile::Volatile;
use super::writer::{ColorCode,Color,ScreenPos};
use lazy_static::lazy_static;

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;
lazy_static!{pub static ref CONSOLE: Mutex<Console> = Mutex::new(Console::new());}

pub const DEFAULT_CHAR:ScreenChar = ScreenChar::from(0x00);

pub struct Console {
    buffer: &'static mut dyn ConsoleBuffer,
}
impl Console {
    pub fn new() -> Self {
        Self {
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        }
    }

    pub fn clear(&mut self) {
        for row in 0..BUFFER_HEIGHT {
            for column in 0..BUFFER_WIDTH {
                self.write_char_at(row, column, DEFAULT_CHAR);
            }
        }
    }
    pub fn remove(&mut self, row:usize, column:usize) {
        self.write_char_at(row,column, DEFAULT_CHAR);
    }
    pub fn write_char_at(&mut self, row:usize, column:usize, chr:ScreenChar) {
        // serial_println!("Printing {} at r:{} c:{}", character.ascii_character as char, row, column);
        self.buffer.write_screenchar_at(&ScreenPos(row, column), chr)
    }
    pub fn write_byte_at(&mut self, row:usize, column:usize, byte:u8) {
        self.write_char_at(row, column, ScreenChar::from(byte))
    }
    pub fn get_at(&mut self, row:usize, column:usize) -> ScreenChar {
        self.buffer.get_screenchar_at(&ScreenPos(row, column))
    }
    pub fn get_atp(&mut self, pos:&ScreenPos) -> ScreenChar {
        self.get_at(pos.0, pos.1)
    }
}
unsafe impl Sync for Console {}
unsafe impl Send for Console {}


pub fn clear_console() {
    CONSOLE.lock().clear()
}


#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct ConsolePos(pub usize, pub usize);


#[derive(Debug)]
pub enum ConsoleError {
    OutOfBounds
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

trait ConsoleBuffer {
    fn get_size(&self) -> (usize,usize);
    fn write_screenchar_at(&mut self, pos:&ScreenPos, chr:ScreenChar);
    fn get_screenchar_at(&self, pos:&ScreenPos) -> ScreenChar;
}

#[repr(transparent)]
#[derive(Debug)]
pub struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT], // [row][column]
}
impl ConsoleBuffer for Buffer {
    fn get_size(&self) -> (usize,usize) {(self.chars.len(), self.chars[0].len())}
    fn write_screenchar_at(&mut self, pos:&ScreenPos, chr:ScreenChar) {self.chars[pos.0][pos.1].write(chr)}
    fn get_screenchar_at(&self, pos:&ScreenPos) -> ScreenChar {self.chars[pos.0][pos.1].read()}
}