use crate::{dbg, drivers::time, ps2, serial_print};

use alloc::boxed::Box;
use pic8259::ChainedPics;
use spin::Mutex;
use x86_64::{
    instructions::port::PortReadOnly,
    structures::{idt::{InterruptDescriptorTable, InterruptStackFrame}, port::PortRead},
};

use super::idt::IDT;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

/// TODO Make a handler for all interrupts and setup a Vec for all interrupts, then we can bind at runtime some functions to be called when this interrupt occurs
/// i.e. when a keyboard interrupt occurs, 3 functions are called, one for the kernel, one that will be passed to userland
pub fn setup_hardware_interrupts(idt: &mut InterruptDescriptorTable) {
    for (i, interrupt) in INTERRUPTS.into_iter() {
        idt[(*i as usize)].set_handler_fn(*interrupt);
    }
}
#[macro_export]
macro_rules! register_interrupt {
    ($num: expr, $func: expr) => {
        $crate::interrupts::hardware::register_interrupt($num, $crate::interrupt_handler!($num, $func).1)
    };
}

pub fn register_interrupt(int_num: InterruptIndex, int: extern "x86-interrupt" fn(x86_64::structures::idt::InterruptStackFrame)) {
    unsafe{IDT.as_mut().unwrap().write()[int_num as usize+PIC_1_OFFSET as usize].set_handler_fn(int)};
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
        crate::interrupt_handler!(InterruptIndex::PS2Mouse, |_| {
            // Do nothing for now, but overriden in mouse.rs
        }),
        crate::interrupt_handler!(InterruptIndex::PrimaryAtaDisk, |_| {
            crate::disk::ata::irq::primary_bus_irq()
        }),
        crate::interrupt_handler!(InterruptIndex::SecondaryAtaDisk, |_| {
            crate::disk::ata::irq::secondary_bus_irq()
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
impl InterruptIndex {
    pub fn from_num_pic(num: u8) -> Option<Self> {
        Some(match num {
            0 => Self::Timer,
            1 => Self::Keyboard,
            3 => Self::COM2,
            4 => Self::COM1,
            5 => Self::LPT2,
            6 => Self::Floppy,
            7 => Self::LPT1,
            8 => Self::CMOS,
            9 => Self::FreeLegacySCSINic,
            10 => Self::FreeSCSINic,
            11 => Self::FreeSCSINic1,
            12 => Self::PS2Mouse,
            13 => Self::FPUCoprocessorInterProcessor,
            14 => Self::PrimaryAtaDisk,
            15 => Self::SecondaryAtaDisk,
            _ => {
                return None
            }
        })
    }
}


// Safe wrapper because the interrupt index should always be valid (if InterruptIndex enum is right...)
pub fn notify_end_of_interrupt(interrupt: InterruptIndex) {
    unsafe {
        PICS.lock().notify_end_of_interrupt(interrupt as u8+32);
    }
}
