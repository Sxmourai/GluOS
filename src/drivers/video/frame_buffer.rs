use spin::{RwLock, RwLockWriteGuard};

use super::Color;


type Buffer = [[volatile::Volatile<u8>; 320]; 200];
pub struct FrameBuffer {
    pub buffer: &'static mut Buffer
}
impl FrameBuffer {
    pub const fn new() -> Self {
        Self {
            buffer: unsafe { &mut *(0xA0000 as *mut Buffer) }
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
    SCREEN.write().buffer[y][x].write(color);
}