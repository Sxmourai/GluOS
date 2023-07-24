use alloc::vec::{self, Vec};

use super::console::{ScreenChar, DEFAULT_CHAR};
use crate::{serial_println, writer::ScreenPos};

pub const BUFFER_HEIGHT: u8 = 25;
pub const BUFFER_WIDTH: u8 = 80;
pub const SBUFFER_HEIGHT: usize = BUFFER_HEIGHT as usize;
pub const SBUFFER_WIDTH: usize = BUFFER_WIDTH as usize;

// ScreenChars buffer
//TODO Make a more generic buffer type
pub trait Buffer {
    type SIZE;
    fn size(&self) -> Self::SIZE;
    fn write_screenchar_at(&mut self, pos: &ScreenPos, chr: ScreenChar);
    fn get_screenchar_at(&self, pos: &ScreenPos) -> ScreenChar;
    fn is_empty(&self) -> bool;
}
// impl dyn Buffer<SIZE = (usize,usize)> {
//     pub fn iter_chars(&self) -> impl Iterator<Item = ScreenChar> {
//         let (width, height) = self.size();
//         for row in 0..height {
//             for column in 0..width {

//             }
//         }
//     }
// }

#[repr(transparent)]
#[derive(Debug)]
pub struct VgaBuffer {
    chars: [[volatile::Volatile<ScreenChar>; SBUFFER_WIDTH]; SBUFFER_HEIGHT], // [row][column]
}
impl Buffer for VgaBuffer {
    type SIZE = (u8, u8);
    fn size(&self) -> Self::SIZE {
        (BUFFER_WIDTH, BUFFER_HEIGHT)
    } // WIDTH, HEIGHT
    fn write_screenchar_at(&mut self, pos: &ScreenPos, chr: ScreenChar) {
        if pos.0 < BUFFER_WIDTH && pos.1 < BUFFER_HEIGHT {
            self.chars[pos.1 as usize][pos.0 as usize].write(chr)
        } else {
            panic!("Tried to write {:?} at {:?}",chr, pos)
        }
    }
    fn get_screenchar_at(&self, pos: &ScreenPos) -> ScreenChar {
        if pos.0 < BUFFER_WIDTH && pos.1 < BUFFER_HEIGHT {
            self.chars[pos.1 as usize][pos.0 as usize].read()
        } else {
            panic!("Tried to read {:?}",pos)
        }
    }
    // Loop over all chars to check if they are DEFAULT_CHARS, so heavy function (O(1))
    fn is_empty(&self) -> bool {
        for y in 0..BUFFER_HEIGHT {
            for x in 0..BUFFER_WIDTH {
                if self.get_screenchar_at(&ScreenPos(x, y)).ascii_character
                    == DEFAULT_CHAR.ascii_character
                {
                    return false;
                }
            }
        }
        true
    }
}

pub struct ConsoleBuffer {
    inner: Vec<[ScreenChar; SBUFFER_WIDTH]>,
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
        //TODO Fix trash code
        let mut inner = alloc::vec![line];
        inner.append(&mut self.inner);
        self.inner = inner;
    }
    pub fn get_youngest_line(&self) -> Option<[ScreenChar; BUFFER_WIDTH as usize]> {
        self.inner.get(self.inner.len() - 1).copied()
    }
    pub fn get_oldest_line(&self) -> Option<[ScreenChar; BUFFER_WIDTH as usize]> {
        self.inner.get(0).copied()
    }
}
impl Buffer for ConsoleBuffer {
    type SIZE = (u8, u8);
    fn size(&self) -> Self::SIZE {
        (BUFFER_WIDTH, self.inner.len().try_into().unwrap())
    }
    fn write_screenchar_at(&mut self, pos: &ScreenPos, chr: ScreenChar) {
        if pos.0 < BUFFER_WIDTH && pos.1 < BUFFER_HEIGHT {
            self.inner[pos.1 as usize][pos.0 as usize] = chr
        } else {
            panic!("Tried to write {:?} at {:?}",chr, pos)
        }
    }
    fn get_screenchar_at(&self, pos: &ScreenPos) -> ScreenChar {
        self.inner[pos.1 as usize][pos.0 as usize]
    }
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}
