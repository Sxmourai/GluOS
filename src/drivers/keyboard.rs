use alloc::{boxed::Box, string::String, vec::Vec};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::{Mutex, RwLock};
use x86_64::{instructions::port::PortReadOnly, structures::idt::InterruptStackFrame};

use crate::{terminal::writer::WRITER, user::prompt::KbInput, print, interrupts::hardware::InterruptIndex};

lazy_static! {
    pub static ref DEFAULT_KEYBOARD: Mutex<KeyboardHandler> = Mutex::new(KeyboardHandler {
        inner: Keyboard::new(
            ScancodeSet1::new(),
            layouts::AnyLayout::Us104Key(layouts::Us104Key),
            HandleControl::Ignore
        ),
        pressed: Vec::new()
    });
}

pub trait KeyboardListener: Sync+Send {
    fn read_scancode(&mut self, scancode: u8);
}
//TODO Do we need thread-safety ?
pub static KEYBOARD_LISTENERS: RwLock<Vec<Box<dyn KeyboardListener>>> = RwLock::new(Vec::new());

pub struct Input {
    pressed: Vec<char>,
}
impl KeyboardListener for Input {
    fn read_scancode(&mut self, scancode: u8) {

        
    }
}




pub struct KeyboardHandler {
    inner: Keyboard<layouts::AnyLayout, ScancodeSet2>,
    pressed: Vec<KeyCode>,
}
impl KeyboardHandler {
    pub fn is_pressed(&self, code: &KeyCode) -> bool {
        self.pressed.contains(code)
    }
    pub fn process_keyevent(&mut self, scancode: u8) -> Option<char> {
        if let Ok(Some(key_event)) = self.inner.add_byte(scancode) {
            let state = key_event.state;
            if state == KeyState::Down {
                self.pressed.push(key_event.code);
            } else if state == KeyState::Up {
                self.pressed.swap_remove(
                    self.pressed
                        .iter()
                        .position(|x| *x == key_event.code)
                        .unwrap(),
                ); //TODO Change .retain to for loop or better (i.e. swap_remove is O(1) but u need the index)
            }
            if let Some(key) = self.inner.process_keyevent(key_event) {
                let mut key_handled = false;
                match key {
                    DecodedKey::RawKey(k) => {
                        match k {
                            KeyCode::ArrowUp => {
                                if self.is_pressed(&KeyCode::LControl) {
                                    WRITER.lock().move_down();
                                    key_handled = true;
                                }
                            }
                            KeyCode::ArrowDown => {
                                if self.is_pressed(&KeyCode::LControl) {
                                    WRITER.lock().move_up();
                                    key_handled = true;
                                }
                            }
                            _ => {}
                        }
                    }
                    DecodedKey::Unicode(k) => {
                        match k {
                            _ => {}
                        }
                    }
                }
                if !key_handled {
                    Some(match key {
                        DecodedKey::RawKey(_) => todo!(),
                        DecodedKey::Unicode(_) => todo!(),
                    })
                }
            }
        }
    }
}