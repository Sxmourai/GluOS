use core::panic;

use alloc::{vec::Vec, format, string::{String, ToString}};
use hashbrown::HashMap;
use pc_keyboard::DecodedKey;

use crate::{writer::{ScreenPos, ColorCode, Color, print_screenchar_at, print_screenchar_atp, print_byte_at, print_atp, print_at, print_screenchars_atp}, println, print, serial_println};
use crate::terminal::console::ScreenChar;

use super::{buffer::BUFFER_WIDTH, console::DEFAULT_CHAR};

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
    fn get_cursor_idx(&self) -> usize; // Returns current cursor index in vec of chars
    fn get_cursor_chr(&self) -> ScreenChar;
    fn get_blinked_chrs(&self) -> &HashMap<ScreenPos, ScreenChar>; // Pointer because a pointer is a lot lighter than hashmap
    fn get_blink_state(&self) -> bool;
    fn get_pressed_keys_len(&self) -> usize;
    fn get_cursor_pos(&self) -> ScreenPos {self.idx_to_pos(self.get_cursor_idx())}
    fn get_return_message(&self) -> String {String::new()} //TODO: For blocking prompt, have to change this
    fn set_blink(&mut self, blink_state:bool);
    fn insert_blink(&mut self, pos:ScreenPos, prev:ScreenChar) -> Option<ScreenChar>;
    fn clear_blinked_chrs(&mut self);
    fn move_cursor(&mut self, pos:usize);
    fn idx_to_pos(&self, idx:usize) -> ScreenPos {
        let origin = self.get_origin();
        ScreenPos(origin.0+(idx/BUFFER_WIDTH-origin.1/BUFFER_WIDTH), (origin.1+idx)%BUFFER_WIDTH)
    }
    fn get_char_at_cursor(&self) -> ScreenChar {crate::writer::WRITER.lock().get_at(self.get_cursor_pos())}
    fn store_previous_cursor(&mut self) {
        let c = self.get_char_at_cursor();
        // if c != self.get_cursor_chr() {
            self.insert_blink(self.get_cursor_pos(), c);
        // }
    }
    fn appear_blink(&self) {print_screenchar_atp(&self.get_cursor_pos(), self.get_cursor_chr());}
    fn restore_blinked(&mut self) {
        for (pos, key) in self.get_blinked_chrs() {
            print_screenchar_atp(pos, *key);
        }
        self.clear_blinked_chrs();
    }
    fn cursor_blink(&mut self) {
        if self.get_blink_state() == false {
            self.store_previous_cursor();
            self.appear_blink(); // Make cursor appear
            self.set_blink(true); // Set state to on
        }
        else {
            self.restore_blinked();
            self.set_blink(false);
        }
    }
    fn remove(&mut self, idx:usize) -> ScreenChar;
    //TODO Change name to something more accurate
    fn rmove_curs_idx(&mut self, relative_idx:isize) { // Neg for left, + for right, 0 none
        if (relative_idx < 0 && (self.get_cursor_idx() as isize)+relative_idx < 0) ||
           (relative_idx+self.get_cursor_idx() as isize > self.get_pressed_keys_len() as isize) 
           {return}
        if relative_idx < 0 {
            self.move_cursor(self.get_cursor_idx() - relative_idx.unsigned_abs());
        } else {
            self.move_cursor(self.get_cursor_idx() + relative_idx as usize);
        }
        self.restore_blinked();
    }
    fn handle_key(&mut self, key:DecodedKey);
    fn get_message(&self) -> String;
}
impl alloc::fmt::Debug for dyn KbInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Input").field("msg", &self.get_message()).finish()
    }
}

// pub struct Prompt {
//     pressed_keys: Vec<ScreenChar>,
//     cursor: Cursor,
//     origin:ScreenPos
// }


// impl Prompt {
//     pub fn new() -> Self {
//         let origin = ScreenPos(0,0);
//         Self {
//             pressed_keys: Vec::new(),
//             cursor: Cursor::new(),
//             origin: origin.clone(),
//         }
//     }
// }

// impl KbInput for Prompt {
//     fn run(self) -> String {
//         crate::interrupts::add_input(self);
//         String::new()
//     }
//     fn get_origin(&self) -> ScreenPos {self.origin.clone()}
//     fn get_cursor_idx(&self) -> usize {self.cursor.pos}
//     fn get_cursor_chr(&self) -> ScreenChar {self.cursor.chr}
//     fn get_blinked_chrs(&self) -> &HashMap<ScreenPos, ScreenChar> {&self.cursor.previous}
//     fn get_blink_state(&self) -> bool {self.cursor.blink_state}
//     fn get_pressed_keys_len(&self) -> usize {self.pressed_keys.len()}
//     fn set_blink(&mut self, blink_state:bool) {self.cursor.blink_state = blink_state}
//     fn clear_blinked_chrs(&mut self) {self.cursor.previous.clear()}
//     fn move_cursor(&mut self, pos:usize) {self.cursor.pos = pos}
//     fn insert_blink(&mut self, pos:ScreenPos, prev:ScreenChar) -> Option<ScreenChar> {self.cursor.previous.insert(pos, prev)}
    
//     fn handle_key(&mut self, key:DecodedKey) {
//         match key {
//             DecodedKey::Unicode(character) => match character {
//                 '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {
//                     if self.cursor.pos > 0 {
//                         print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
//                         self.rmove_curs_idx(-1);
//                         self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
//                         print_atp(&self.origin, &self.pressed_keys);
//                     }
//                 }), // Backspace
//                 '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
//                     if self.cursor.pos < self.pressed_keys.len() {
//                         print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
//                         self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
//                         print_atp(&self.origin, &self.pressed_keys);
//                     }
//                 }), // Delete
//                 '\t' => {}, // Tab
//                 '\u{1b}' => {crate::vga_buffer::clear_screen(); print_byte_at(5, 5, 0x48)}, // Escape
//                 _ => {
//                     let c = ScreenChar::from(character as u8);
//                     print_screenchar_atp(&self.get_cursor_pos(), &c);
//                     self.store_previous_cursor();
//                     self.pressed_keys.insert(self.cursor.pos, c);
//                     if self.cursor.pos != self.pressed_keys.len() { // Push elements
//                         for (i, chr) in &mut self.pressed_keys[self.cursor.pos..].iter().enumerate() {
//                             print_screenchar_at(self.get_cursor_pos().0+i/BUFFER_WIDTH, self.cursor.pos+i%BUFFER_WIDTH, chr);
//                         }
//                     }
//                     self.rmove_curs_idx(1);
//                     if self.cursor.blink_state == true {self.appear_blink();}
//                     else {print_screenchar_atp(&self.get_cursor_pos(), &ScreenChar::from(0x00));}
//                 },
//             },
//             DecodedKey::RawKey(key) => match key {
//                 pc_keyboard::KeyCode::ArrowLeft => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos > 0 {self.rmove_curs_idx(-1);}}),
//                 pc_keyboard::KeyCode::ArrowRight => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos < self.pressed_keys.len() {self.rmove_curs_idx(1);}}),
//                 _ => println!("{:?}", key)
//             }
//         }
//     }
// }


// impl core::fmt::Debug for Prompt {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         f.debug_struct("Prompt").field("pressed_keys", &self.pressed_keys).field("cursor_pos", &self.cursor.pos).finish()
//     }
// }

struct BlockingPrompt {
    message:String,
    pressed_keys: Vec<ScreenChar>,
    cursor: Cursor,
    origin:ScreenPos,
    pub return_message: String
}

impl BlockingPrompt {
    pub fn new(message:&str) -> Self {
        Self {
            message: message.to_string(),
            pressed_keys: Vec::new(),
            cursor: Cursor::new(),
            origin: ScreenPos(0,0),
            return_message: String::new()
        }
    }
    fn handle_end(&mut self) {
        self.return_message = self.pressed_keys.iter().map(|sc| sc.ascii_character as char).collect();
        self.return_message.push(' '); // Flag for the run fn
    }
}
impl KbInput for BlockingPrompt {

    fn get_message(&self) -> String {self.message.to_string()}
    fn run(mut self) -> String {
        self.origin = crate::writer::calculate_end(&crate::writer::WRITER.lock().cursor_pos.clone(), &self.message); // Clone to be sure to not lock it during the function
        print!("{}", self.message);
        let idx = crate::interrupts::add_input(self);
        loop {
            if let Some(mut msg) = crate::interrupts::get_input_msg(idx) {
                if msg.ends_with(' ') {
                    crate::interrupts::remove_input(idx).as_mut().0.restore_blinked();
                    if msg.remove(msg.len()-1) != ' ' {panic!("ERROR: {}", msg)}
                    return msg;
                }
            } else {panic!("Trying to get an input that doesn't exist, index isn't right:{}",idx)}
            x86_64::instructions::hlt(); // Halts until next hardware interrupt, can be time or keyboard
        }
    }
    fn get_return_message(&self) -> String {self.return_message.to_string()}
    fn get_origin(&self) -> ScreenPos {self.origin.clone()}
    fn get_cursor_idx(&self) -> usize {self.cursor.pos}
    fn get_cursor_chr(&self) -> ScreenChar {self.cursor.chr}
    fn get_blinked_chrs(&self) -> &HashMap<ScreenPos, ScreenChar> {&self.cursor.previous}
    fn get_blink_state(&self) -> bool {self.cursor.blink_state}
    fn get_pressed_keys_len(&self) -> usize {self.pressed_keys.len()}
    fn set_blink(&mut self, blink_state:bool) {self.cursor.blink_state = blink_state}
    fn clear_blinked_chrs(&mut self) {self.cursor.previous.clear()}
    fn move_cursor(&mut self, pos:usize) {self.cursor.pos = pos}
    fn insert_blink(&mut self, pos:ScreenPos, prev:ScreenChar) -> Option<ScreenChar> {self.cursor.previous.insert(pos, prev)}

    fn handle_key(&mut self, key:DecodedKey) {
        match key {
            DecodedKey::Unicode(character) => match character {
                '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {// Backspace
                    if self.cursor.pos > 0 {
                        // print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() );
                        
                        self.rmove_curs_idx(-1);
                        self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }), 
                '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.cursor.pos < self.pressed_keys.len() {
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() );
                        self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
                        print_screenchars_atp(&self.origin, self.pressed_keys.clone());
                    }
                }), // Delete
                '\t' => {}, // Tab
                '\n' => self.handle_end(), 
                '\u{1b}' => {crate::terminal::clear(); print_byte_at(5, 5, 0x48)}, // Escape
                _ => {
                    let c = ScreenChar::from(character as u8);
                    print_screenchar_atp(&self.get_cursor_pos(), c);
                    self.store_previous_cursor();
                    self.pressed_keys.insert(self.cursor.pos, c);
                    if self.cursor.pos != self.pressed_keys.len()-1 { // Push elements
                        for (i, chr) in &mut self.pressed_keys[self.cursor.pos..].iter().enumerate() {
                            print_screenchar_at(self.get_cursor_pos().0+i/BUFFER_WIDTH, self.cursor.pos+i%BUFFER_WIDTH, *chr);
                        }
                    }
                    self.rmove_curs_idx(1);
                    if self.get_blink_state()==true {self.appear_blink()}
                    // else {print_screenchar_atp(&self.get_cursor_pos(), DEFAULT_CHAR)}
                },
            },
            DecodedKey::RawKey(key) => match key {
                pc_keyboard::KeyCode::ArrowLeft => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos > 0 {self.rmove_curs_idx(-1);}}),
                pc_keyboard::KeyCode::ArrowRight => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos < self.pressed_keys.len() {self.rmove_curs_idx(1);}}),
                _ => {}
            }
        }
    }

    fn remove(&mut self, idx:usize) -> ScreenChar {self.pressed_keys.remove(idx)}
}

pub fn input(message:&str) -> String {
    crate::prompt::BlockingPrompt::new(message).run()
}
