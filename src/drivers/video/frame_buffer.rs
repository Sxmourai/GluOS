use core::ptr::write_volatile;

use spin::{RwLock, RwLockWriteGuard};

use super::Color;
pub struct VgaLinearBuffer {
    pub inner: [[u8; 320]; 200]
}

pub struct FrameBuffer {
    pub buffer: &'static mut VgaLinearBuffer
}
impl FrameBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { &mut *(0xA0000 as *mut VgaLinearBuffer) }
        }
    }
    
}
pub const SW:u16 = 320;
pub const SH:u8 =  200;
lazy_static::lazy_static! {
    pub static ref SCREEN: RwLock<FrameBuffer> = RwLock::new(FrameBuffer::new());
}
pub type ScreenLock = RwLockWriteGuard<'static, FrameBuffer>;

pub fn fill_rect(x:usize, y: usize, w:usize, h:usize, color: Color) {
}
pub fn put_pixel(x:usize, y: usize, color: Color) {
    put_pixel_lock(x, y, color, &mut SCREEN.write())
}
pub fn put_pixel_lock(x:usize, y:usize, color: Color, screen: &mut ScreenLock) {
    unsafe { write_volatile(&mut screen.buffer.inner[y][x], color) }
}