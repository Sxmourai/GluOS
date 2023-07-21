use alloc::vec::{Vec, self};

use crate::writer::ScreenPos;
use super::console::{ScreenChar, DEFAULT_CHAR};

pub const BUFFER_HEIGHT: usize = 25;
pub const BUFFER_WIDTH: usize = 80;


// ScreenChars buffer
//TODO Make a more generic buffer type
pub trait Buffer {
    type SIZE;
    fn size(&self) -> Self::SIZE;
    fn write_screenchar_at(&mut self, pos:&ScreenPos, chr:ScreenChar);
    fn get_screenchar_at(&self, pos:&ScreenPos) -> ScreenChar;
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
    chars: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT], // [row][column]
}
impl Buffer for VgaBuffer {
    type SIZE = (usize,usize);
    fn size(&self) -> Self::SIZE {(BUFFER_WIDTH, BUFFER_HEIGHT)} // WIDTH, HEIGHT
    fn write_screenchar_at(&mut self, pos:&ScreenPos, chr:ScreenChar) {self.chars[pos.0][pos.1] = chr}
    fn get_screenchar_at(&self, pos:&ScreenPos) -> ScreenChar {self.chars[pos.0][pos.1]}
    // Loop over all chars to check if they are DEFAULT_CHARS, so heavy function (O(1))
    fn is_empty(&self) -> bool {
        for row in 0..BUFFER_HEIGHT {
            for column in 0..BUFFER_WIDTH {
                if self.get_screenchar_at(&ScreenPos(row, column)) == DEFAULT_CHAR {
                    return false;
                }
            }
        }
        true
    }
}

pub struct ConsoleBuffer {
    inner: Vec<[ScreenChar;BUFFER_WIDTH]>
}
impl ConsoleBuffer {
    pub fn new() -> Self {
        Self{inner: Vec::new()}
    }
    // appends item at end of buffer
    pub fn append(&mut self, line: [ScreenChar;BUFFER_WIDTH]) {self.inner.push(line)}
    // pushes item at start of buffer and moves everything else
    pub fn push(&mut self, line: [ScreenChar;BUFFER_WIDTH]) { //TODO Fix trash code
        let mut inner = alloc::vec![line];
        inner.append(&mut self.inner);
        self.inner = inner;
    }
    pub fn get_youngest_line(&self) -> Option<[ScreenChar;BUFFER_WIDTH]> {self.inner.get(self.inner.len()-1).copied()}
    pub fn get_oldest_line(&self) -> Option<[ScreenChar;BUFFER_WIDTH]> {self.inner.get(0).copied()}
}
impl Buffer for ConsoleBuffer {
    type SIZE = (usize,usize);
    fn size(&self) -> Self::SIZE {(BUFFER_WIDTH, self.inner.len())}
    fn write_screenchar_at(&mut self, pos:&ScreenPos, chr:ScreenChar) {self.inner[pos.0][pos.1] = chr}
    fn get_screenchar_at(&self, pos:&ScreenPos) -> ScreenChar {self.inner[pos.0][pos.1]}
    fn is_empty(&self) -> bool {self.inner.is_empty()}
}