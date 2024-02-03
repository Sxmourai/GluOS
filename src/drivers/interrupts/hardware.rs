use crate::drivers::time;


use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

pub fn setup_hardware_interrupts(idt: &mut InterruptDescriptorTable) {
    for (i,interrupt) in INTERRUPTS.into_iter().enumerate() {
        idt[i+32].set_handler_fn(*interrupt);
    }
}

pub const INTERRUPTS: &[extern "x86-interrupt" fn(InterruptStackFrame)] = &[
    timer, keyboard, cascade,
];


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET+1,
    Cascade = PIC_1_OFFSET+2,
}


#[macro_export]
macro_rules! interrupt_handler {
    ($idx: expr, $f: expr, $name: tt) => {
        pub extern "x86-interrupt" fn $name(stack_frame: x86_64::structures::idt::InterruptStackFrame) {
            $f(stack_frame);
            crate::interrupts::hardware::notify_end_of_interrupt($idx);
        }
    };
}

crate::interrupt_handler!(InterruptIndex::Timer, |_stack_frame| {
    
}, timer);
crate::interrupt_handler!(InterruptIndex::Cascade, |_stack_frame| {
    
}, cascade);


// Safe wrapper because the interrupt index should always be valid (if InterruptIndex enum is right...)
fn notify_end_of_interrupt(interrupt: InterruptIndex) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(interrupt as u8);
    }
}

const KEYBOARD_DATA_PORT: PortReadOnly<u8> = PortReadOnly::new(0x60);
pub extern "x86-interrupt" fn keyboard(_stack_frame: InterruptStackFrame) {
    #[allow(const_item_mutation)]
    let scancode: u8 = unsafe { KEYBOARD_DATA_PORT.read() };
    crate::task::keyboard::DEFAULT_KEYBOARD
        .lock()
        .process_keyevent(scancode);

    notify_end_of_interrupt(InterruptIndex::Keyboard)
}

pub extern "x86-interrupt" fn second_interrupt_controller(_stack_frame: InterruptStackFrame) {
    use crate::x86_64::instructions::port::PortRead;
    unsafe{log::debug!("{:?} | {:?}", _stack_frame, (u8::read_from_port(0x20),u8::read_from_port(0x21),u8::read_from_port(0xa0),u8::read_from_port(0xa1)))};
    notify_end_of_interrupt(InterruptIndex::Cascade)
}
