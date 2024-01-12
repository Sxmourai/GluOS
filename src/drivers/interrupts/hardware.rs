use crate::{drivers::time};


use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::{
        idt::{InterruptStackFrame},
    },
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
    SecondInterruptController = PIC_2_OFFSET + 4,
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
fn notify_end_of_interrupt(interrupt: InterruptIndex) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(interrupt.as_u8());
    }
}

pub extern "x86-interrupt" fn timer(_stack_frame: InterruptStackFrame) {
    time::tick();

    notify_end_of_interrupt(InterruptIndex::Timer);
}
const KEYBOARD_DATA_PORT: PortReadOnly<u8> = PortReadOnly::new(0x60);
pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    #[allow(const_item_mutation)]
    let scancode: u8 = unsafe { KEYBOARD_DATA_PORT.read() };
    crate::task::keyboard::DEFAULT_KEYBOARD
        .lock()
        .process_keyevent(scancode);

    notify_end_of_interrupt(InterruptIndex::Keyboard)
}

pub extern "x86-interrupt" fn second_interrupt_controller(_stack_frame: InterruptStackFrame) {
    // unsafe{log::debug!("{:?} | {:?}", _stack_frame, (inb(0x20),inb(0x21),inb(0xa0),inb(0xa1)))};
    notify_end_of_interrupt(InterruptIndex::SecondInterruptController)
}
