use alloc::{string::String, boxed::Box, vec::Vec};
use pc_keyboard::{Keyboard, layouts::Us104Key, ScancodeSet1, HandleControl};
use pic8259::ChainedPics;
use spin::Mutex;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug)]
pub struct SendSyncWrapper<T: ?Sized>(pub T);
unsafe impl<T: ?Sized> Sync for SendSyncWrapper<T> {}
unsafe impl<T: ?Sized> Send for SendSyncWrapper<T> {}

static KB_INPUTS: Mutex<Vec<Box<SendSyncWrapper<dyn KbInput>>>> = Mutex::new(Vec::new());

// Adds prompt to list and returns its index
pub fn add_input(input:impl KbInput + 'static) -> usize {
    KB_INPUTS.lock().push(Box::new(SendSyncWrapper(input)));
    KB_INPUTS.lock().len()-1
}
// Removes prompt from list and returns it
pub fn remove_input(idx:usize) -> Box<SendSyncWrapper<dyn KbInput>> {
    KB_INPUTS.lock().remove(idx)
}
pub fn get_input_msg(idx:usize) -> Option<String> {
    if let Some(input) = KB_INPUTS.lock().get(idx) {
        return Some(input.0.get_return_message())
    } 
    None
}
lazy_static::lazy_static!{
    static ref KEYBOARD: Mutex<Keyboard<Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(Us104Key, ScancodeSet1, HandleControl::Ignore));
} 


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

pub extern "x86-interrupt" fn timer(_stack_frame: InterruptStackFrame) {
    crate::timer::tick();
    
    if crate::timer::get_ticks()%14==0 { // Blinking
        for prompt in KB_INPUTS.lock().iter_mut() { 
            prompt.0.cursor_blink();
        }
    }
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}


pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    // crate::task::keyboard::add_scancode(scancode);
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            drop(keyboard);
            for input in KB_INPUTS.lock().iter_mut() {
                input.0.handle_key(key);
            }
        }
    }
    


    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::structures::idt::{PageFaultErrorCode, InterruptStackFrame};
use crate::{boot::hlt_loop, prompt::KbInput};
