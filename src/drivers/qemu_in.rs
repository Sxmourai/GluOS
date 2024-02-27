use alloc::string::String;
use x86_64::{instructions::hlt, structures::port::PortRead};

use crate::{dbg, time::sdelay};

pub struct QemuIOReader {
    inputted: String,
}

impl Default for QemuIOReader {
    fn default() -> Self {
        return Self::new()
    }
}

impl QemuIOReader {
    pub fn new() -> Self {
        return Self {
            inputted: String::new(),
        }
    }
    pub async fn run(mut self) {
        loop {
            let k = unsafe { u8::read_from_port(0x3F8) };
            if k == 13 {
                crate::dbg!(k);
                return;
            }
            if k != 0 {
                dbg!(k)
            }
        }
    }
}
