use alloc::{boxed::Box, string::String, vec::Vec};
use pc_keyboard::{layouts::Us104Key, DecodedKey, HandleControl, KeyCode, Keyboard, ScancodeSet1};
use pic8259::ChainedPics;
use spin::Mutex;
use crate::{boot::hlt_loop, prompt::KbInput, serial_println, writer::{WRITER, inb}, pci::port::Port, drivers::{get_driver, time}};
use x86_64::structures::{idt::{InterruptStackFrame, PageFaultErrorCode}, port::PortRead};


pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}
// Safe wrapper because the interrupt index should always be valid (if InterruptIndex enum is right...)
fn notify_end_of_interrupt(interrupt:InterruptIndex) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(interrupt.as_u8());
    }
}

pub extern "x86-interrupt" fn timer(_stack_frame: InterruptStackFrame) {
    time::tick();

    notify_end_of_interrupt(InterruptIndex::Timer);
}

pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    let scancode: u8 = unsafe { inb(0x60) };
    crate::task::keyboard::DEFAULT_KEYBOARD.lock().process_keyevent(scancode);

    notify_end_of_interrupt(InterruptIndex::Keyboard)
}
