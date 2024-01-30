pub mod exceptions;
pub mod hardware;
pub mod idt;
#[cfg(feature="apic")]
pub mod apic;
pub mod msr;
pub mod multiprocessor;

pub fn init() {
    idt::IDT.load(); // Init the interrupt descriptor table, handling cpu exceptions
    unsafe { hardware::PICS.lock().initialize() }; // Init pic, for hardware interrupts (Time, Keyboard...)
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
}
