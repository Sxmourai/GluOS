use crate::{serial_println, terminal::buffer::VgaBuffer, writer::{print_screenchar_at, print_screenchars_atp}};
use alloc::vec::Vec;
use spin::Mutex;

use super::{
    buffer::{Buffer, ConsoleBuffer, BUFFER_HEIGHT, BUFFER_WIDTH},
    writer::{Color, ColorCode, ScreenPos},
};
use lazy_static::lazy_static;


pub const DEFAULT_CHAR: ScreenChar = ScreenChar::new('\0' as u8, ColorCode(15)); // Black on black

pub struct Console {
    pub buffer: &'static mut dyn Buffer<SIZE = (u8, u8)>,
    pub top_buffer: ConsoleBuffer,
    pub bottom_buffer: ConsoleBuffer,
}
impl Console {
    pub fn new(buffer: &'static mut dyn Buffer<SIZE = (u8, u8)>) -> Self {
        Self {
            buffer,
            top_buffer: ConsoleBuffer::new(),
            bottom_buffer: ConsoleBuffer::new(),
        }
    }

    pub fn write_char_at(&mut self, x: u8, y: u8, chr: ScreenChar) {
        self.buffer.write_screenchar_at(&ScreenPos(x, y), chr)
    }

    pub fn write_byte_at(&mut self, x: u8, y: u8, byte: u8) {
        self.write_char_at(x, y, ScreenChar::from(byte))
    }

    pub fn clear(&mut self) {
        for y in 0..self.size().1 {
            for x in 0..self.size().0 {
                self.remove(x, y);
            }
        }
        self.top_buffer = ConsoleBuffer::new(); // Don't use clear because the allocated size doesn't change
        self.bottom_buffer = ConsoleBuffer::new(); // Don't use clear because the allocated size doesn't change
        // Could use clear then shrink to fit I think
        //TODO: Find out which is faster (even tho I don't think it will be a gigantic improvement)
    }

    pub fn remove(&mut self, x: u8, y: u8) {
        self.write_char_at(x, y, DEFAULT_CHAR);
    }

    pub fn get_at(&self, x: u8, y: u8) -> ScreenChar {
        self.buffer.get_screenchar_at(&ScreenPos(x, y))
    }
    pub fn get_atp(&self, pos: &ScreenPos) -> ScreenChar {
        self.get_at(pos.0, pos.1)
    }
    // Note that this makes a copy
    //TODO Support top and bottom buffer ?
    pub fn get_str_at(&self, pos: &ScreenPos, len: u16) -> Vec<ScreenChar> {
        let mut buffer = Vec::new();
        let (width, height) = self.buffer.size();
        for i in 0..len {
            buffer.push(self.get_at(
                (pos.0 as u16 + i) as u8 % width,
                ((i/width as u16) as u8 + pos.1),
            )); // Wrap around
        }
        buffer
    }
    pub fn size(&self) -> (u8, u8) {
        self.buffer.size()
    }
    // pub fn iter_chars(&self) -> impl Iterator<Item = ScreenChar> {self.buffer.}
}
unsafe impl Sync for Console {}
unsafe impl Send for Console {}

pub fn clear_console() {
    print_screenchars_atp(&ScreenPos(0, 0), [DEFAULT_CHAR; 80*25])
}

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct ConsolePos(pub usize, pub usize);

#[derive(Debug)]
pub enum ConsoleError {
    OutOfBounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(C)]
pub struct ScreenChar {
    pub ascii_character: u8,
    pub color_code: ColorCode,
}
impl ScreenChar {
    pub const fn new(ascii_character: u8, color_code: ColorCode) -> Self {
        Self {
            ascii_character,
            color_code,
        }
    }
    pub const fn from(ascii_character: u8) -> Self {
        Self::new(ascii_character,ColorCode::new(Color::White, Color::Black))
    }
    pub const fn default() -> Self {
        Self::from(0x00)
    }
}
