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

pub struct Prompt {
    keyboard: Keyboard<Us104Key, ScancodeSet1>,
    pressed_keys: Vec<ScreenChar>,
    cursor: Cursor,
    origin:ScreenPos
}
impl Prompt {
    pub fn new() -> Self {
        let origin = ScreenPos(0,0);
        Self {
            keyboard: Keyboard::new(Us104Key, ScancodeSet1, HandleControl::Ignore),
            pressed_keys: Vec::new(),
            cursor: Cursor::new(),
            origin: origin.clone(),
        }
    }
    pub fn run(self) {
        crate::interrupts::add_prompt(self);
    }
    fn get_cursor_pos(&self) -> ScreenPos {self.idx_to_pos(self.cursor.pos)}
    fn idx_to_pos(&self, idx:usize) -> ScreenPos {
        ScreenPos(self.origin.0+(idx/BUFFER_WIDTH)-self.origin.1, (self.origin.1+idx)%BUFFER_WIDTH)
    }
    fn get_char_at_cursor(&self) -> ScreenChar {crate::vga_buffer::WRITER.lock().get_atp(self.get_cursor_pos())}
    pub fn store_previous_cursor(&mut self) {
        let c = self.get_char_at_cursor();
        if c != self.cursor.chr {
            self.cursor.previous.insert(self.get_cursor_pos(), c);
        }
    }
    pub fn appear_blink(&self) {print_screenchar_atp(&self.get_cursor_pos(), &self.cursor.chr);}
    pub fn restore_blinked(&mut self) {
        for (pos, key) in &self.cursor.previous {
            print_screenchar_atp(pos, key);
        }
        self.cursor.previous.clear();
    }
    pub fn cursor_blink(&mut self) {
        if self.cursor.blink_state == false {
            self.store_previous_cursor();
            self.appear_blink(); // Make cursor appear
            self.cursor.blink_state = true; // Set state to on
        }
        else {
            self.restore_blinked();
            self.cursor.blink_state = false;
        }
    }
    fn end(&mut self) {
        serial_println!("Pressed: {:?}\nPrev: {:?}", self.pressed_keys.iter().map(|k| {k.ascii_character as char}).collect::<Vec<char>>(),self.cursor.previous.iter().map(|k| {(k.1.ascii_character as char, k.0)}).collect::<Vec<(char, &ScreenPos)>>());
    }
    fn remove_at(&mut self, idx:usize) -> ScreenChar {
        // print_byte_atp(self.idx_to_pos(idx), 0x00);
        self.pressed_keys.remove(idx)
    }
    pub fn press_key(&mut self, key:u8) {
        if let Ok(Some(key_event)) = self.keyboard.add_byte(key) {
            if let Some(key) = self.keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => match character {
                        '\u{8}' => x86_64::instructions::interrupts::without_interrupts(|| {
                            if self.cursor.pos > 0 {
                                print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
                                self.rmove_curs_idx(-1);
                                self.remove_at(self.cursor.pos); //TODO: FIX HORRIBLE CODE
                                print_atp(&self.origin, &self.pressed_keys);
                            }
                        }), // Backspace
                        '\u{7f}' => x86_64::instructions::interrupts::without_interrupts(|| {
                            if self.cursor.pos < self.pressed_keys.len() {
                                print_at(self.origin.0, self.origin.1, format!("{}",0x00 as char).repeat(self.pressed_keys.len()).as_str() ).unwrap();
                                self.remove_at(self.cursor.pos); //TODO: FIX HORRIBLE CODE
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
    }
    // pub fn get_pressed(&self) -> &Vec<char> {
    //     &self.pressed_keys
    // }
    // fn move_cursor(&mut self, direction: InputDirection) {
    //     if direction == InputDirection::Left {
    //         if self.cursor_pos.1 > 0 {
    //             self.cursor_pos.1 -= 1;
    //         } else {
    //             self.cursor_pos.1 = BUFFER_WIDTH-1;
    //             self.cursor_pos.0 -= 1;
    //         }
    //     } else {
    //         if self.cursor_pos.1+1 < BUFFER_WIDTH { // Could do len-1 but if buffer empty, substract overflow (0-1 in unsigned)
    //             self.cursor_pos.1 += 1;
    //         } else {
    //             self.cursor_pos.1 = 0;
    //             self.cursor_pos.0 += 1;
    //         }
    //     }
    // }
    fn rmove_curs_idx(&mut self, relative_idx:isize) { // Neg for left, + for right, 0 none
        if (relative_idx < 0 && (self.cursor.pos as isize)+relative_idx < 0) ||
           (relative_idx+self.cursor.pos as isize > self.pressed_keys.len() as isize) 
           {return}
        if relative_idx < 0 {
            self.cursor.pos -= relative_idx.abs() as usize;
        } else {
            self.cursor.pos += relative_idx as usize;
        }
        self.restore_blinked();
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
    pub fn run(&self) -> String {
        print!("{}", self.message);
        let p = Prompt::new();
        p.origin = calculate_end(&ScreenPos(0, 0), &self.message);
        p.run();
    }
}