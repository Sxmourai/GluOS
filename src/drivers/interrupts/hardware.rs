use crate::{dbg, drivers::time, ps2, serial_print};

use alloc::boxed::Box;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::{idt::{InterruptDescriptorTable, InterruptStackFrame}, port::PortRead},
};

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

extern "x86-interrupt" fn panic_interrupt(stack_frame: InterruptStackFrame) {
    crate::serial_println!("Unknown interrupt: {:?}", stack_frame);
    panic!()
}

pub fn setup_hardware_interrupts(idt: &mut InterruptDescriptorTable) {
    for (i, interrupt) in INTERRUPTS.into_iter() {
        idt[(*i as usize)].set_handler_fn(*interrupt);
    }
}


pub const INTERRUPTS: &[(u8, extern "x86-interrupt" fn(x86_64::structures::idt::InterruptStackFrame,))] = {
    &[
        crate::interrupt_handler!(InterruptIndex::Timer, |_stack_frame| {
        }),
        crate::interrupt_handler!(InterruptIndex::Keyboard, |_stack_frame| {
            #[allow(const_item_mutation)]
            let scancode: u8 = unsafe { ps2::DATA_PORT.read() };
            crate::task::keyboard::DEFAULT_KEYBOARD
                .lock()
                .process_keyevent(scancode);
        }),
        crate::interrupt_handler!(InterruptIndex::PS2Mouse, |stack_frame| {
            dbg!("Mouse interrupt !!", stack_frame);
        }),
    ]
};
#[macro_export]
macro_rules! interrupt_handler {
    ($idx: expr, $f: expr) => {{
        let interrupt_num = 32+$idx as u8;
        pub extern "x86-interrupt" fn _int(
            stack_frame: x86_64::structures::idt::InterruptStackFrame,
        ) {
            #[allow(clippy::redundant_closure_call)]
            $f(stack_frame);
            $crate::interrupts::hardware::notify_end_of_interrupt($idx);
        }
        (interrupt_num, _int)
    }}
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = 0,
    Keyboard = 1,
    COM2 = 3,
    COM1 = 4,
    LPT2 = 5,
    Floppy = 6,
    /// Or Unreliable "spurious" interrupt
    LPT1 = 7,
    CMOS = 8,
    FreeLegacySCSINic = 9,
    FreeSCSINic = 10,
    FreeSCSINic1 = 11,
    PS2Mouse = 12,
    FPUCoprocessorInterProcessor=13,
    PrimaryAtaDisk=14,
    SecondaryAtaDisk=15,
}



// Safe wrapper because the interrupt index should always be valid (if InterruptIndex enum is right...)
fn notify_end_of_interrupt(interrupt: InterruptIndex) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(interrupt as u8+32);
    }
}
