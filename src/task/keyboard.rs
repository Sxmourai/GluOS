use crate::{println, vga_buffer::ScreenChar};
use alloc::vec::Vec;
use alloc::vec;
use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use lazy_static::lazy_static;
use spin::Mutex;
use core::{pin::Pin, task::{Poll, Context}};
use futures_util::stream::Stream;
use futures_util::task::AtomicWaker;
use futures_util::stream::StreamExt;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use crate::print;

static WAKER: AtomicWaker = AtomicWaker::new();
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

/// Called by the keyboard interrupt handler
///
/// Must not block or allocate.
pub(crate) fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE.try_init_once(|| ArrayQueue::new(100))
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

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

struct Blinker {
    state: bool,
    cursor:ScreenChar,
    previous:ScreenChar // Previous character that was replaced
}
impl Blinker {
    pub fn new(cursor: ScreenChar) -> Blinker {
        Blinker { state: false, cursor: cursor, previous: cursor }
    }
    pub fn from(cursor:char) -> Blinker {
        Blinker::new(ScreenChar::from(cursor as u8))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum InputDirection{
    Left,Right
}
pub struct Input {
    keyboard: Keyboard<layouts::Us104Key, ScancodeSet1>,
    pressed_keys: Vec<char>,
    blink: Blinker,
    // callback: alloc::boxed::Box<dyn Fn() -> ()>,
    cursor_pos:usize
}
impl core::fmt::Debug for Input {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Input").field("pressed_keys", &self.pressed_keys).field("cursor_pos", &self.cursor_pos).finish()
    }
}


impl Input {
    pub fn new() -> Input {
        Input {
            keyboard: Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore),
            pressed_keys: Vec::new(),
            // callback: alloc::boxed::Box::new(callback),
            blink: Blinker::from('A'), // First previous is useless
            cursor_pos: 0
        }
    }
    pub fn blink(&mut self) {
        let row = crate::vga_buffer::BUFFER_HEIGHT-1;
        let column = self.cursor_pos;
        if self.blink.state == false {
            self.blink.previous = crate::vga_buffer::WRITER.lock().get_at(row,column);
            crate::vga_buffer::WRITER.lock().write_at(row, column, self.blink.cursor);
            self.blink.state = true;
        }
        else {
            crate::vga_buffer::WRITER.lock().write_at(row, column, self.blink.previous);
            self.blink.state = false;
        }
    }

    pub fn press_key(&mut self, key:u8) {
        if let Ok(Some(key_event)) = self.keyboard.add_byte(key) {
            if let Some(key) = self.keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => match character {
                        '\u{8}' => {self.pressed_keys.remove(self.cursor_pos+1); self.move_cursor(InputDirection::Left)}, // Backspace
                        '\u{7f}' => {self.pressed_keys.remove(self.cursor_pos);}, // In brackets to ignore return value // Delete
                        // '\n' => self.callback(), // Enter
                        '\t' => {}, // Tab
                        '\u{1b}' => {crate::vga_buffer::clear_screen(); crate::vga_buffer::WRITER.lock().write_at(5, 5, ScreenChar::from(0x48))}, // Escape
                        _ => {print!("{}", character); self.pressed_keys.push(character)},
                    },
                    DecodedKey::RawKey(key) => match key {
                        pc_keyboard::KeyCode::ArrowLeft => self.move_cursor(InputDirection::Left),
                        pc_keyboard::KeyCode::ArrowRight => self.move_cursor(InputDirection::Right),
                        pc_keyboard::KeyCode::End => self.move_cursor_to(self.pressed_keys.len()-1),
                        _ => println!("{:?}", key)
                    }
                }
            }
        }
    }
    pub fn get_pressed(&self) -> &Vec<char> {
        &self.pressed_keys
    }
    fn move_cursor(&mut self, direction: InputDirection) {
        if direction == InputDirection::Left && self.cursor_pos > 0 {
            self.cursor_pos -= 1;
        } else if direction == InputDirection::Right && self.cursor_pos < self.pressed_keys.len()-1 {
            self.cursor_pos += 1;
        } 
    }
    fn move_cursor_to(&mut self, pos: usize) {
        self.cursor_pos = pos 
    }
}
