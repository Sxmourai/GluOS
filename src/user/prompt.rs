use core::panic;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use pc_keyboard::{DecodedKey, KeyCode};
use spin::RwLock;

use crate::terminal::{
    buffer::{BUFFER_WIDTH, SBUFFER_WIDTH},
    console::{ScreenChar, DEFAULT_CHAR},
    writer::{print_at, print_screenchar_at, print_screenchars_atp, ScreenPos},
};
use crate::{print, println};

pub static mut COMMANDS_HISTORY: RwLock<Vec<Vec<ScreenChar>>> = RwLock::new(Vec::new());
pub static mut COMMANDS_INDEX: RwLock<usize> = RwLock::new(0);

pub trait KbInput: Send + Sync {
    fn run(self) -> String;
    fn get_origin(&self) -> ScreenPos;
    fn get_pressed_keys_len(&self) -> usize;
    fn get_return_message(&self) -> String {
        String::new()
    } //TODO: For blocking prompt, have to change this
    fn move_cursor(&mut self, pos: usize);

    fn remove(&mut self, idx: usize) -> ScreenChar;
    fn handle_key(&mut self, key: DecodedKey);
    fn get_message(&self) -> String;
}
impl alloc::fmt::Debug for dyn KbInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Input")
            .field("msg", &self.get_message())
            .finish()
    }
}

struct BlockingPrompt {
    message: String,
    pressed_keys: Vec<ScreenChar>,
    pos: usize,
    origin: ScreenPos,
    pub return_message: String,
}

impl BlockingPrompt {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
            pressed_keys: Vec::new(),
            pos: 0,
            origin: ScreenPos(0, 0), // 0 re-set in run
            return_message: String::new(),
        }
    }
    fn handle_end(&mut self) {
        println!();
        self.return_message = self
            .pressed_keys
            .iter()
            .map(|sc| sc.ascii_character as char)
            .collect();
        self.return_message.push(' '); // Flag for the run fn
    }
    pub fn cursor_pos(&self) -> ScreenPos {
        ScreenPos(
            (self.pos / BUFFER_WIDTH as usize) as u8,
            (self.pos % SBUFFER_WIDTH) as u8,
        )
    }
}
impl KbInput for BlockingPrompt {
    fn move_cursor(&mut self, pos: usize) {
        self.pos = pos;
        crate::terminal::writer::WRITER.lock().move_cursor(
            ((self.pos + self.origin.0 as usize) % SBUFFER_WIDTH) as u8,
            (self.pos / SBUFFER_WIDTH) as u8 + self.origin.1,
        )
    }
    fn get_message(&self) -> String {
        self.message.to_string()
    }
    fn run(mut self) -> String {
        self.origin = crate::terminal::writer::calculate_end(
            &crate::terminal::writer::WRITER.lock().pos.clone(),
            self.message.len(),
        ); // Clone to be sure to not lock it during the function
        print!("{}", self.message);
        let idx = crate::task::keyboard::add_input(self);
        loop {
            if let Some(mut msg) = crate::task::keyboard::get_input_msg(idx) {
                if msg.ends_with(' ') {
                    crate::task::keyboard::remove_input(idx);
                    if msg.remove(msg.len() - 1) != ' ' {
                        panic!("ERROR: {}", msg)
                    }
                    return msg;
                }
            // let scancode = crate::terminal::serial::read_serial_input();
            // if scancode!=0 {
            //     crate::task::keyboard::DEFAULT_KEYBOARD
            //     .lock()
            //     .process_keyevent(scancode);
            // }
            } else {
                panic!(
                    "Trying to get an input that doesn't exist, index isn't right:{}",
                    idx
                )
            }
            x86_64::instructions::hlt(); // Halts until next hardware interrupt, can be time or keyboard
        }
    }
    fn get_return_message(&self) -> String {
        self.return_message.to_string()
    }
    fn get_origin(&self) -> ScreenPos {
        self.origin.clone()
    }
    fn get_pressed_keys_len(&self) -> usize {
        self.pressed_keys.len()
    }
    fn handle_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::Unicode(character) => match character {
                '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    // Backspace
                    if self.pos > 0 {
                        //TODO: Fix the fact that we clear entire input every time we backspace
                        // Remove all chars of line
                        // serial_println!("Or{:?}", self.origin);
                        print_at(
                            self.origin.0,
                            self.origin.1,
                            format!("{}", 0x00 as char)
                                .repeat(self.pressed_keys.len())
                                .as_str(),
                        );
                        self.move_cursor(self.pos - 1);
                        self.remove(self.pos);
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }),
                '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.pos < self.pressed_keys.len() {
                        //TODO: Same as backspace
                        print_at(
                            self.origin.0,
                            self.origin.1,
                            format!("{}", 0x00 as char)
                                .repeat(self.pressed_keys.len())
                                .as_str(),
                        );
                        self.remove(self.pos);
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }), // Delete
                '\t' => {} // Tab
                '\n' => self.handle_end(),
                '\u{1b}' => crate::terminal::clear(), // Escape
                _ => {
                    let c = ScreenChar::from(character as u8);

                    print_screenchar_at(
                        (self.origin.0 as usize + self.pos % SBUFFER_WIDTH) as u8,
                        ((self.pos) / SBUFFER_WIDTH) as u8 + self.origin.1,
                        c,
                    );
                    self.pressed_keys.insert(self.pos, c);
                    if self.pos < self.pressed_keys.len() - 1 {
                        // Push elements
                        for (i, chr) in &mut self.pressed_keys[self.pos..].iter().enumerate() {
                            print_screenchar_at(
                                ((self.pos + i + self.origin.0 as usize) % SBUFFER_WIDTH) as u8,
                                ((self.pos + i + self.origin.0 as usize) / SBUFFER_WIDTH) as u8
                                    + self.origin.1,
                                *chr,
                            );
                        }
                    }
                    self.move_cursor(self.pos + 1);
                }
            },
            DecodedKey::RawKey(key) => match key {
                KeyCode::ArrowLeft => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.pos > 0 {
                        self.move_cursor(self.pos - 1);
                    }
                }),
                KeyCode::ArrowRight => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.pos < self.pressed_keys.len() {
                        self.move_cursor(self.pos + 1);
                    }
                }),
                KeyCode::ArrowUp => {
                    if unsafe { *COMMANDS_INDEX.read() } > 0 {
                        *unsafe { COMMANDS_INDEX.write() } -= 1;
                        if unsafe { *COMMANDS_INDEX.read() }
                            == unsafe { COMMANDS_HISTORY.read().len() } - 1
                        {
                            unsafe { COMMANDS_HISTORY.write().push(self.pressed_keys.clone()) };
                        }
                        {
                            let history = unsafe { COMMANDS_HISTORY.read() };
                            let last_command =
                                history.get(unsafe { *COMMANDS_INDEX.read() }).unwrap();
                            self.pressed_keys = last_command.clone();
                        }
                        print_screenchars_atp(&self.origin, [DEFAULT_CHAR; 70]);
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                        self.move_cursor(self.get_pressed_keys_len());
                    }
                }
                KeyCode::ArrowDown => {
                    if unsafe { *COMMANDS_INDEX.read() } + 1
                        < unsafe { COMMANDS_HISTORY.read().len() }
                    {
                        *unsafe { COMMANDS_INDEX.write() } += 1;
                        {
                            let history = unsafe { COMMANDS_HISTORY.read() };
                            let last_command =
                                history.get(unsafe { *COMMANDS_INDEX.read() }).unwrap();
                            self.pressed_keys = last_command.clone();
                        }
                        print_screenchars_atp(&self.origin, [DEFAULT_CHAR; 70]);
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                        self.move_cursor(self.get_pressed_keys_len());
                    }
                }
                _ => {}
            },
        }
    }

    fn remove(&mut self, idx: usize) -> ScreenChar {
        self.pressed_keys.remove(idx)
    }
}

pub fn input(message: &str) -> String {
    crate::user::prompt::BlockingPrompt::new(message).run()
}
