use core::panic;

use alloc::{vec::Vec, format, string::{String, ToString}};
use hashbrown::HashMap;
use pc_keyboard::DecodedKey;

use crate::{writer::{ScreenPos, ColorCode, Color, print_screenchar_at, print_screenchar_atp, print_byte_at, print_atp, print_at, print_screenchars_atp}, println, print, serial_println};
use crate::terminal::console::ScreenChar;

use super::{buffer::{BUFFER_WIDTH, SBUFFER_WIDTH}, console::DEFAULT_CHAR};

#[derive(Debug)]
pub struct Cursor {
    blink_state: bool,
    chr:ScreenChar,
    previous:HashMap<ScreenPos, ScreenChar>,
    pub pos: usize
}
impl Cursor {
    pub fn new() -> Cursor {
        Cursor { 
            blink_state: false, 
            chr: ScreenChar::new(b' ', ColorCode::new(Color::White, Color::White)), 
            previous: HashMap::new(),
            pos: 0
        }
    }
}
pub trait KbInput: Send + Sync {
    fn run(self) -> String;
    fn get_origin(&self) -> ScreenPos;
    fn get_pressed_keys_len(&self) -> usize;
    fn get_return_message(&self) -> String {String::new()} //TODO: For blocking prompt, have to change this
    fn move_cursor(&mut self, pos:usize);

    fn remove(&mut self, idx:usize) -> ScreenChar;
    fn handle_key(&mut self, key:DecodedKey);
    fn get_message(&self) -> String;
}
impl alloc::fmt::Debug for dyn KbInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Input").field("msg", &self.get_message()).finish()
    }
}

struct BlockingPrompt {
    message:String,
    pressed_keys: Vec<ScreenChar>,
    pos: usize,
    origin:ScreenPos,
    pub return_message: String
}

impl BlockingPrompt {
    pub fn new(message:&str) -> Self {
        Self {
            message: message.to_string(),
            pressed_keys: Vec::new(),
            pos: 0,
            origin: ScreenPos(0,0), // 0 re-set in run
            return_message: String::new()
        }
    }
    fn handle_end(&mut self) {
        println!();
        self.return_message = self.pressed_keys.iter().map(|sc| sc.ascii_character as char).collect();
        self.return_message.push(' '); // Flag for the run fn
    }
    fn cursor_pos(&self) -> ScreenPos {
        ScreenPos((self.pos/BUFFER_WIDTH as usize) as u8, (self.pos%SBUFFER_WIDTH) as u8)
    }
}
impl KbInput for BlockingPrompt {
    fn move_cursor(&mut self, pos:usize) {
        self.pos = pos;
        crate::writer::WRITER.lock().move_cursor(((self.pos+self.origin.0 as usize)%SBUFFER_WIDTH) as u8, (self.pos/SBUFFER_WIDTH) as u8 + self.origin.1)
    }
    fn get_message(&self) -> String {self.message.to_string()}
    fn run(mut self) -> String {
        self.origin = crate::writer::calculate_end(&crate::writer::WRITER.lock().pos.clone(), self.message.len()); // Clone to be sure to not lock it during the function
        print!("{}", self.message);
        let idx = crate::interrupts::add_input(self);
        loop {
            if let Some(mut msg) = crate::interrupts::get_input_msg(idx) {
                if msg.ends_with(' ') {
                    crate::interrupts::remove_input(idx);
                    if msg.remove(msg.len()-1) != ' ' {panic!("ERROR: {}", msg)}
                    return msg;
                }
            } else {panic!("Trying to get an input that doesn't exist, index isn't right:{}",idx)}
            x86_64::instructions::hlt(); // Halts until next hardware interrupt, can be time or keyboard
        }
    }
    fn get_return_message(&self) -> String {self.return_message.to_string()}
    fn get_origin(&self) -> ScreenPos {self.origin.clone()}
    fn get_pressed_keys_len(&self) -> usize {self.pressed_keys.len()}
    fn handle_key(&mut self, key:DecodedKey) {
        match key {
            DecodedKey::Unicode(character) => match character {
                '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {// Backspace
                    if self.pos > 0 {//TODO: Fix the fact that we clear entire input every time we backspace
                        // Remove all chars of line
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() );
                        self.move_cursor(self.pos-1);
                        self.remove(self.pos); 
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }), 
                '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.pos < self.pressed_keys.len() {//TODO: Same as backspace
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() );
                        self.remove(self.pos);
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }), // Delete
                '\t' => {}, // Tab
                '\n' => self.handle_end(), 
                '\u{1b}' => crate::terminal::clear(), // Escape
                _ => {
                    let c = ScreenChar::from(character as u8);

                    print_screenchar_at((self.origin.0 as usize+self.pos%SBUFFER_WIDTH) as u8, ((self.pos)/SBUFFER_WIDTH) as u8+self.origin.1, c);
                    self.pressed_keys.insert(self.pos, c);
                    if self.pos < self.pressed_keys.len()-1 { // Push elements
                        for (i, chr) in &mut self.pressed_keys[self.pos..].iter().enumerate() {
                            print_screenchar_at(((self.pos+i+self.origin.0 as usize)%SBUFFER_WIDTH) as u8, (((self.pos+i+self.origin.0 as usize)/SBUFFER_WIDTH) as u8 + self.origin.1), *chr);
                        }
                    }
                    self.move_cursor(self.pos+1);
                },
            },
            DecodedKey::RawKey(key) => match key {
                pc_keyboard::KeyCode::ArrowLeft => x86_64::instructions::interrupts::without_interrupts(|| {if self.pos > 0 {self.move_cursor(self.pos-1);}}),
                pc_keyboard::KeyCode::ArrowRight => x86_64::instructions::interrupts::without_interrupts(|| {if self.pos < self.pressed_keys.len() {self.move_cursor(self.pos+1);}}),
                _ => {}
            }
        }
    }

    fn remove(&mut self, idx:usize) -> ScreenChar {self.pressed_keys.remove(idx)}
}

pub fn input(message:&str) -> String {
    crate::prompt::BlockingPrompt::new(message).run()
}
