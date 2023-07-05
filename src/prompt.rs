use alloc::{vec::Vec, format, string::String};
use hashbrown::HashMap;
use pc_keyboard::{Keyboard, layouts::Us104Key, ScancodeSet1, HandleControl, DecodedKey};

use crate::{vga_buffer::{ScreenChar, ScreenPos, ColorCode, Color, BUFFER_WIDTH, print_screenchar_at, print_screenchar_atp, print_byte_at, print_char_at, print_byte_atp, print_atp, print_at, calculate_end}, serial_println, println, print};


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
            chr: ScreenChar::new(' ' as u8, ColorCode::new(Color::White, Color::White)), 
            previous: HashMap::new(),
            pos: 0
        }
    }

}
pub trait KbInput {
    fn run(self) -> Option<String>;
    fn get_origin(&self) -> ScreenPos;
    fn get_cursor_idx(&self) -> usize; // Returns current cursor index in vec of chars
    fn get_cursor_chr(&self) -> ScreenChar;
    fn get_blinked_chrs(&self) -> &HashMap<ScreenPos, ScreenChar>; // Pointer because a pointer is a lot lighter than hashmap
    fn get_blink_state(&self) -> bool;
    fn get_pressed_keys_len(&self) -> usize;
    fn get_cursor_pos(&self) -> ScreenPos {self.idx_to_pos(self.get_cursor_idx())}
    fn set_blink(&mut self, blink_state:bool);
    fn clear_blinked_chrs(&mut self);
    fn move_cursor(&mut self, pos:usize);
    fn idx_to_pos(&self, idx:usize) -> ScreenPos {
        let origin = self.get_origin();
        ScreenPos(origin.0+(idx/BUFFER_WIDTH)-origin.1, (origin.1+idx)%BUFFER_WIDTH)
    }
    fn get_char_at_cursor(&self) -> ScreenChar {crate::vga_buffer::WRITER.lock().get_atp(self.get_cursor_pos())}
    fn store_previous_cursor(&mut self) {
        let c = self.get_char_at_cursor();
        if c != self.get_cursor_chr() {
            self.get_blinked_chrs().insert(self.get_cursor_pos(), c);
        }
    }
    fn appear_blink(&self) {print_screenchar_atp(&self.get_cursor_pos(), &self.get_cursor_chr());}
    fn restore_blinked(&mut self) {
        for (pos, key) in self.get_blinked_chrs() {
            print_screenchar_atp(pos, key);
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
    fn remove(&mut self, idx:usize) -> ScreenChar {
        self.remove(idx)
    }
    
    fn rmove_curs_idx(&mut self, relative_idx:isize) { // Neg for left, + for right, 0 none
        if (relative_idx < 0 && (self.get_cursor_idx() as isize)+relative_idx < 0) ||
           (relative_idx+self.get_cursor_idx() as isize > self.get_pressed_keys_len() as isize) 
           {return}
        if relative_idx < 0 {
            self.move_cursor(self.get_cursor_idx() - relative_idx.abs() as usize);
        } else {
            self.move_cursor(self.get_cursor_idx() + relative_idx as usize);
        }
        self.restore_blinked();
    }
    fn handle_key(&mut self, key:DecodedKey);
}

pub struct Prompt {
    pressed_keys: Vec<ScreenChar>,
    cursor: Cursor,
    origin:ScreenPos
}


impl Prompt {
    pub fn new() -> Self {
        let origin = ScreenPos(0,0);
        Self {
            pressed_keys: Vec::new(),
            cursor: Cursor::new(),
            origin: origin.clone(),
        }
    }
    pub fn cursor_blink(&mut self) {KbInput::cursor_blink(self)}
    fn end(&mut self){}
}

impl KbInput for Prompt {
    fn run(self) -> Option<String> {
        crate::interrupts::add_input(self);
        None
    }
    fn get_origin(&self) -> ScreenPos {self.origin}
    fn get_cursor_idx(&self) -> usize {self.cursor.pos}
    fn get_cursor_chr(&self) -> ScreenChar {self.cursor.chr}
    fn get_blinked_chrs(&self) -> &HashMap<ScreenPos, ScreenChar> {&self.cursor.previous}
    fn get_blink_state(&self) -> bool {self.cursor.blink_state}
    fn get_pressed_keys_len(&self) -> usize {self.pressed_keys.len()}
    fn set_blink(&mut self, blink_state:bool) {self.cursor.blink_state = blink_state}
    fn clear_blinked_chrs(&mut self) {self.cursor.previous.clear()}
    fn move_cursor(&mut self, pos:usize) {self.cursor.pos = pos}
    
    fn handle_key(&mut self, key:DecodedKey) {
        match key {
            DecodedKey::Unicode(character) => match character {
                '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.cursor.pos > 0 {
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
                        self.rmove_curs_idx(-1);
                        self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
                        print_atp(&self.origin, &self.pressed_keys);
                    }
                }), // Backspace
                '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
                    if self.cursor.pos < self.pressed_keys.len() {
                        print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
                        self.remove(self.cursor.pos); //TODO: FIX HORRIBLE CODE
                        print_atp(&self.origin, &self.pressed_keys);
                    }
                }), // Delete
                '\n' => x86_64::instructions::interrupts::without_interrupts(|| {self.end()}), // Enter
                '\t' => {}, // Tab
                '\u{1b}' => {crate::vga_buffer::clear_screen(); print_byte_at(5, 5, 0x48)}, // Escape
                _ => {
                    let c = ScreenChar::from(character as u8);
                    print_screenchar_atp(&self.get_cursor_pos(), &c);
                    self.store_previous_cursor();
                    self.pressed_keys.insert(self.cursor.pos, c);
                    if self.cursor.pos != self.pressed_keys.len() { // Push elements
                        for (i, chr) in &mut self.pressed_keys[self.cursor.pos..].iter().enumerate() {
                            print_screenchar_at(self.get_cursor_pos().0+i/BUFFER_WIDTH, self.cursor.pos+i%BUFFER_WIDTH, chr);
                        }
                    }
                    self.rmove_curs_idx(1);
                    if self.cursor.blink_state == true {self.appear_blink();}
                    else {print_screenchar_atp(&self.get_cursor_pos(), &ScreenChar::from(0x00));}
                },
            },
            DecodedKey::RawKey(key) => match key {
                pc_keyboard::KeyCode::ArrowLeft => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos > 0 {self.rmove_curs_idx(-1);}}),
                pc_keyboard::KeyCode::ArrowRight => x86_64::instructions::interrupts::without_interrupts(|| {if self.cursor.pos < self.pressed_keys.len() {self.rmove_curs_idx(1);}}),
                pc_keyboard::KeyCode::End => x86_64::instructions::interrupts::without_interrupts(|| {self.end()}),
                _ => println!("{:?}", key)
            }
        }
    }
}


impl core::fmt::Debug for Prompt {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Prompt").field("pressed_keys", &self.pressed_keys).field("cursor_pos", &self.cursor.pos).finish()
    }
}

pub struct BlockingPrompt {
    message:String
}

impl BlockingPrompt {
    pub fn new(message:String) -> Self {
        Self {
            message
        }
    }
    // pub fn run(&self) -> Option<String> {
    //     print!("{}", self.message);
    //     let p = Prompt::new();
    //     p.origin = calculate_end(&ScreenPos(0, 0), &self.message);
    //     p.run();
    // }
}