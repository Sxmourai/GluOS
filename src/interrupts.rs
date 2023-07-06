use crate::{alloc::{vec::Vec, boxed::Box}, serial_println};
use alloc::string::String;
use pc_keyboard::{ScancodeSet1, Keyboard, layouts::Us104Key, HandleControl};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use crate::{println, gdt, prompt::{Cursor, KbInput}};
use lazy_static::lazy_static;
use pic8259::ChainedPics;

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt[InterruptIndex::Timer.as_usize()]
            .set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()]
            .set_handler_fn(keyboard_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler); 
        idt
    };
}
pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(
    stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}


// HARDWARE INTERRUPTS
use spin::{self, Mutex};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug)]
struct SendSyncWrapper<T: ?Sized>(T);
unsafe impl<T: ?Sized> Sync for SendSyncWrapper<T> {}
unsafe impl<T: ?Sized> Send for SendSyncWrapper<T> {}

static KB_INPUTS: Mutex<Vec<Box<SendSyncWrapper<dyn KbInput>>>> = Mutex::new(Vec::new());

// Adds prompt to list and returns its index
pub fn add_input(input:impl KbInput + 'static) -> usize {
    KB_INPUTS.lock().push(Box::new(SendSyncWrapper(input)));
    KB_INPUTS.lock().len()-1
}
pub fn get_input_msg(idx:usize) -> Option<String> {
    if let Some(input) = KB_INPUTS.lock().get(idx) {
        return Some(input.0.get_return_message())
    } 
    None
}

lazy_static!{
    static ref KEYBOARD: Mutex<Keyboard<Us104Key, ScancodeSet1>> = Mutex::new(Keyboard::new(Us104Key, ScancodeSet1, HandleControl::Ignore));
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

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: InterruptStackFrame) {
    crate::timer::tick();
    
    if crate::timer::get_ticks()%14==0 {
        for prompt in KB_INPUTS.lock().iter_mut() { 
            prompt.0.cursor_blink();
        }
    }
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}


extern "x86-interrupt" fn keyboard_interrupt_handler(
    _stack_frame: InterruptStackFrame
) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    // crate::task::keyboard::add_scancode(scancode);
    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            drop(keyboard);
            for input in KB_INPUTS.lock().iter_mut() {
                input.0.handle_key(key);
            }
        }
    }
    


    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

use x86_64::structures::idt::PageFaultErrorCode;
use crate::boot::hlt_loop;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

// qemu-system-x86_64 -drive format=raw,file=target/x86_64/debug/bootimage-kernel.bin -no-reboot -device isa-debug-exit,iobase=0xf4,iosize=0x04 -serial stdio -display none