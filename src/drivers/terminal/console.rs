use core::ptr::{read_volatile, write_volatile};

use alloc::vec::Vec;

use super::{
    buffer::{ConsoleBuffer, VgaBuffer},
    writer::{print_screenchars_atp, Color, ColorCode, ScreenPos},
};

pub const DEFAULT_CHAR: ScreenChar = ScreenChar::new(b'\0', ColorCode(15)); // Black on black

pub struct Console {
    pub buffer: &'static mut VgaBuffer,
    pub top_buffer: ConsoleBuffer,
    pub bottom_buffer: ConsoleBuffer,
}
impl Console {
    pub fn new(buffer: &'static mut VgaBuffer) -> Self {
        return Self {
            buffer,
            top_buffer: ConsoleBuffer::new(),
            bottom_buffer: ConsoleBuffer::new(),
        }
    }

    pub fn write_char_at(&mut self, x: u8, y: u8, chr: ScreenChar) {
        if x < self.size().0 && y < self.size().1 {
            unsafe { write_volatile(&mut self.buffer.chars[y as usize][x as usize], chr) }
        } else {
            log::error!("Tried to write {:?} at {:?}", chr, (x, y))
        }
    }
    pub fn get_char_at(&self, x: u8, y: u8) -> ScreenChar {
        if x < self.size().0 && y < self.size().1 {
            unsafe { return read_volatile(&self.buffer.chars[y as usize][x as usize]) }
        } else {
            log::error!("Tried to read {:?}", (x, y));
            return DEFAULT_CHAR
        }
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
        self.top_buffer.inner.clear();
        self.top_buffer.inner.shrink_to(2); // Sets up a 2 lines capacity... This is for speed but idk if it's that important
        self.bottom_buffer.inner.clear();
        self.bottom_buffer.inner.shrink_to(2);
    }

    pub fn remove(&mut self, x: u8, y: u8) {
        self.write_char_at(x, y, DEFAULT_CHAR);
    }
    //Doesn't support top and bottom buffer because we ScreenPos is u8's, where the max value is 256, which means we won't be able to read a lot from the buffers
    pub fn get_str_at(&self, pos: &ScreenPos, len: u16) -> &'static [ScreenChar] {
        let (width, _height) = self.size();
        let mut first_char = core::ptr::addr_of!(self.buffer) as *const ScreenChar;
        first_char = unsafe { first_char.add(width as usize * pos.1 as usize + pos.0 as usize) };
        unsafe { return core::slice::from_raw_parts(first_char, len as usize) }
    }
    pub fn size(&self) -> (u8, u8) {
        return (super::buffer::BUFFER_WIDTH, super::buffer::BUFFER_HEIGHT)
    }
}
unsafe impl Sync for Console {}
unsafe impl Send for Console {}

pub fn clear_console() {
    print_screenchars_atp(&ScreenPos(0, 0), [DEFAULT_CHAR; 80 * 25])
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
        return Self {
            ascii_character,
            color_code,
        }
    }
    pub const fn from(ascii_character: u8) -> Self {
        return Self::new(ascii_character, ColorCode::new(Color::White, Color::Black))
    }
    pub const fn default() -> Self {
        return Self::from(0x00)
    }
}
