use alloc::{vec::Vec, string::String, boxed::Box};
use conquer_once::spin::OnceCell;
use spin::Mutex;
use core::{
    pin::Pin,
    task::{Context, Poll}, ops::Index,
};
use crossbeam_queue::ArrayQueue;
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;
use lazy_static::lazy_static;
use pc_keyboard::{layouts, HandleControl, Keyboard, ScancodeSet1, KeyCode, DecodedKey, KeyState};

use crate::{serial_println, prompt::KbInput, writer::WRITER, log::print_trace};

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
lazy_static! {
    pub static ref DEFAULT_KEYBOARD: Mutex<KeyboardHandler> = Mutex::new(
        KeyboardHandler {
            inner: Keyboard::new(
            ScancodeSet1::new(),
            layouts::AnyLayout::Us104Key(layouts::Us104Key),
            HandleControl::Ignore),
            pressed: Vec::new()
        }
    );
}

static KB_INPUTS: Mutex<Vec<Box<SendSyncWrapper<dyn KbInput>>>> = Mutex::new(Vec::new());
pub struct KeyboardHandler {
    inner: Keyboard<layouts::AnyLayout, ScancodeSet1>,
    pressed: Vec<KeyCode>,
}
impl KeyboardHandler {
    pub fn is_pressed(&self, code: &KeyCode) -> bool {self.pressed.contains(code)}

    pub fn process_keyevent(&mut self, scancode:u8) {
        if let Ok(Some(key_event)) = self.inner.add_byte(scancode) {
            let state = key_event.state;
            if let Some(key) = self.inner.process_keyevent(key_event) {
                for input in KB_INPUTS.lock().iter_mut() {
                    input.0.handle_key(key);
                }
                match key {
                    DecodedKey::RawKey(k) => {
                        if state == KeyState::Down {
                            self.pressed.push(k);
                        }else if state == KeyState::Up {
                            self.pressed.retain(|x| *x != k); //TODO Change .retain to for loop or better (i.e. swap_remove is O(1) but u need the index)
                        }
                        match k {
                            KeyCode::ArrowUp => if self.is_pressed(&KeyCode::LControl) {WRITER.lock().move_down()},
                            KeyCode::ArrowDown => if self.is_pressed(&KeyCode::LControl) {WRITER.lock().move_up()},
                            KeyCode::T => print_trace(),
                            _ => {} //serial_println!("Unsupported key: {:?}", k),
                        }
                    },
                    DecodedKey::Unicode(k) => {
                        match k {
                            't' => print_trace(),
                            _ => {serial_println!("This shouldn't happen because HandleControl is at ignore... {:?}", k);}
                        }
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

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
// pub(crate) fn add_scancode(scancode: u8) {
//     if let Ok(queue) = SCANCODE_QUEUE.try_get() {
//         if let Err(_) = queue.push(scancode) {
//             println!("WARNING: scancode queue full; dropping keyboard input");
//         } else {
//             WAKER.wake();
//         }
//     } else {
//         println!("WARNING: scancode queue uninitialized");
//     }
// }

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}
