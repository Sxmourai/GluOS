use alloc::{boxed::Box, string::String, vec::Vec};
use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, KeyCode, KeyState, Keyboard, ScancodeSet1};
use spin::Mutex;

use crate::{terminal::writer::WRITER, user::prompt::KbInput};

// static WAKER: AtomicWaker = AtomicWaker::new();
// static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

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

static KB_INPUTS: Mutex<Vec<Box<SendSyncWrapper<dyn KbInput>>>> = Mutex::new(Vec::new());
pub struct KeyboardHandler {
    inner: Keyboard<layouts::AnyLayout, ScancodeSet1>,
    pressed: Vec<KeyCode>,
}
impl KeyboardHandler {
    pub fn is_pressed(&self, code: &KeyCode) -> bool {
        self.pressed.contains(code)
    }

    pub fn process_keyevent(&mut self, scancode: u8) {
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
                    DecodedKey::RawKey(k) => match k {
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
                    },
                    DecodedKey::Unicode(k) => {}
                }
                if !key_handled {
                    for input in KB_INPUTS.lock().iter_mut() {
                        input.0.handle_key(key);
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct SendSyncWrapper<T: ?Sized>(pub T);
unsafe impl<T: ?Sized> Sync for SendSyncWrapper<T> {}
unsafe impl<T: ?Sized> Send for SendSyncWrapper<T> {}

// Adds prompt to list and returns its index
pub fn add_input(input: impl KbInput + 'static) -> usize {
    KB_INPUTS.lock().push(Box::new(SendSyncWrapper(input)));
    KB_INPUTS.lock().len() - 1
}
// Removes prompt from list and returns it
pub fn remove_input(idx: usize) -> Box<SendSyncWrapper<dyn KbInput>> {
    KB_INPUTS.lock().remove(idx)
}
pub fn get_input_msg(idx: usize) -> Option<String> {
    if let Some(input) = KB_INPUTS.lock().get(idx) {
        return Some(input.0.get_return_message());
    }
    None
}
