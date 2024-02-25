use alloc::vec::Vec;

use super::{console::ScreenChar, writer::ScreenPos};

pub const BUFFER_HEIGHT: u8 = 25;
pub const BUFFER_WIDTH: u8 = 80;
pub const SBUFFER_HEIGHT: usize = BUFFER_HEIGHT as usize;
pub const SBUFFER_WIDTH: usize = BUFFER_WIDTH as usize;

/// A buffer of Screenchar
pub trait Buffer {
    type SIZE;
    fn size(&self) -> Self::SIZE;
    fn write_screenchar_at(&mut self, pos: &ScreenPos, chr: ScreenChar);
    fn get_screenchar_at(&self, pos: &ScreenPos) -> ScreenChar;
    fn is_empty(&self) -> bool;
}

#[repr(transparent)]
#[derive(Debug)]
pub struct VgaBuffer {
    pub chars: [[ScreenChar; SBUFFER_WIDTH]; SBUFFER_HEIGHT], // [row][column]
}

pub struct ConsoleBuffer {
    pub inner: Vec<[ScreenChar; SBUFFER_WIDTH]>,
}
impl Default for ConsoleBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsoleBuffer {
    pub fn new() -> Self {
        Self { inner: Vec::new() }
    }
    // appends item at end of buffer
    pub fn append(&mut self, line: [ScreenChar; SBUFFER_WIDTH]) {
        self.inner.push(line)
    }
    // pushes item at start of buffer and moves everything else
    pub fn push(&mut self, line: [ScreenChar; SBUFFER_WIDTH]) {
        self.inner.insert(0, line);
    }
    pub fn get_youngest_line(&self) -> Option<[ScreenChar; BUFFER_WIDTH as usize]> {
        self.inner.last().copied()
    }
    pub fn remove_youngest_line(&mut self) -> [ScreenChar; BUFFER_WIDTH as usize] {
        self.inner.remove(self.inner.len() - 1)
    }
    pub fn get_oldest_line(&self) -> Option<[ScreenChar; BUFFER_WIDTH as usize]> {
        self.inner.first().copied()
    }

    pub fn size(&self) -> (u8, u8) {
        (BUFFER_WIDTH, self.inner.len().try_into().unwrap())
    }
    pub fn write_screenchar_at(&mut self, pos: &ScreenPos, chr: ScreenChar) {
        if pos.0 < BUFFER_WIDTH && pos.1 < BUFFER_HEIGHT {
            self.inner[pos.1 as usize][pos.0 as usize] = chr
        } else {
            panic!("Tried to write {:?} at {:?}", chr, pos)
        }
    }
    pub fn get_screenchar_at(&self, pos: &ScreenPos) -> ScreenChar {
        self.inner[pos.1 as usize][pos.0 as usize]
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
