use alloc::vec::Vec;
use spin::Mutex;
use volatile::Volatile;
use crate::{terminal::buffer::VgaBuffer, serial_println};

use super::{writer::{ColorCode,Color,ScreenPos}, buffer::{Buffer, BUFFER_HEIGHT, BUFFER_WIDTH, ConsoleBuffer}};
use lazy_static::lazy_static;
lazy_static!{pub static ref CONSOLE: Mutex<Console> = Mutex::new(Console::new(unsafe { &mut *(0xb8000 as *mut VgaBuffer) }));}

pub const DEFAULT_CHAR:ScreenChar = ScreenChar::from(0x00);

pub struct Console {
    pub buffer: &'static mut dyn Buffer<SIZE = (usize,usize)>,
    pub top_buffer: ConsoleBuffer,
    pub bottom_buffer: ConsoleBuffer, 
}
impl Console {
    pub fn new(buffer: &'static mut dyn Buffer<SIZE = (usize,usize)>) -> Self {
        Self {
            buffer,
            top_buffer: ConsoleBuffer::new(),
            bottom_buffer: ConsoleBuffer::new(),
        }
    }

    pub fn write_char_at(&mut self, row:usize, column:usize, chr:ScreenChar) {
        // serial_println!("Printing {} at r:{} c:{}", character.ascii_character as char, row, column);
        
        self.buffer.write_screenchar_at(&ScreenPos(row, column), chr)
    }
    
    pub fn write_byte_at(&mut self, row:usize, column:usize, byte:u8) {
        self.write_char_at(row, column, ScreenChar::from(byte))
    }

    pub fn clear(&mut self) {
        for row in 0..self.size().0 {
            for column in 0..self.size().1 {
                self.write_char_at(row, column, DEFAULT_CHAR);
            }
        }
        self.top_buffer = ConsoleBuffer::new(); // Don't use clear because the allocated size doesn't change
        self.bottom_buffer = ConsoleBuffer::new(); // Don't use clear because the allocated size doesn't change
        // Could use clear then shrink to fit I think
        //TODO: Find out which is faster (even tho I don't think it will be a gigantic improvement)
    }
    
    pub fn remove(&mut self, row:usize, column:usize) {
        self.write_char_at(row,column, DEFAULT_CHAR);
    }

    pub fn get_at(&self, row:usize, column:usize) -> ScreenChar {
        self.buffer.get_screenchar_at(&ScreenPos(row, column))
    }
    pub fn get_atp(&self, pos:&ScreenPos) -> ScreenChar {
        self.get_at(pos.0, pos.1)
    }
    // Note that this makes a copy
    pub fn get_str_at(&self, pos:&ScreenPos, len:usize) -> Vec<ScreenChar> {
        let mut buffer = Vec::new();
        let (width, height) = self.buffer.size();
        for i in 0..len {
            buffer.push(self.get_at(pos.0+i/width, (pos.1+i)%width)); // Wrap around
        }
        buffer
    }
    pub fn size(&self) -> (usize,usize) {self.buffer.size()}
    // pub fn iter_chars(&self) -> impl Iterator<Item = ScreenChar> {self.buffer.}
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

pub fn pretty_print() -> !{
    let buffer = &mut CONSOLE.lock().buffer;
    let mut i:usize = 256;
    loop {
        i += 1;
        for row in 0..buffer.size().1 {
            for column in 0..buffer.size().0 {
                buffer.write_screenchar_at(&ScreenPos(row, column), ScreenChar { ascii_character: (row as u8).wrapping_add(i as u8), color_code: ColorCode::newb((column as f32*2.7)as u8, 0) })
            }
        }
            // x86_64::instructions::hlt();x86_64::instructions::hlt();x86_64::instructions::hlt();x86_64::instructions::hlt();
    }
}