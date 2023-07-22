pub mod idt;
pub mod exceptions;
pub mod hardware;
pub mod timer;

pub use hardware::{add_input,get_input_msg,remove_input};

pub fn init() {
    idt::IDT.load(); // Init the interrupt descriptor table, handling cpu exceptions
    unsafe { hardware::PICS.lock().initialize() }; // Init pic, for hardware interrupts (Time, Keyboard...)
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
}