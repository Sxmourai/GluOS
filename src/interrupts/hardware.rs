use alloc::{string::String, boxed::Box, vec::Vec};
use pc_keyboard::{Keyboard, layouts::Us104Key, ScancodeSet1, HandleControl, DecodedKey, KeyCode};
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
    let mut keyboard = crate::task::keyboard::DEFAULT_KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) { // Could drop keyboard, but only this function should use it, so for now it's fine
            for input in KB_INPUTS.lock().iter_mut() {
                input.0.handle_key(key);
            }
            match key {
                DecodedKey::RawKey(k) => match k {
                    KeyCode::ArrowUp => WRITER.lock().move_down(),
                    KeyCode::ArrowDown => WRITER.lock().move_up(),
                    _ => {}, //serial_println!("Unsupported key: {:?}", k),
                },
                DecodedKey::Unicode(k) => {}
            }
            
        }
    }
    


    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::structures::idt::{PageFaultErrorCode, InterruptStackFrame};
use crate::{boot::hlt_loop, prompt::KbInput, writer::WRITER, serial_println};
