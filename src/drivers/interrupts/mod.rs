use spin::RwLock;

#[cfg(feature = "apic")]
pub mod apic;
pub mod exceptions;
pub mod hardware;
pub mod idt;
pub mod irq;
pub mod msr;
pub mod multiprocessor;

pub fn init() {
    unsafe { idt::IDT.replace(RwLock::new(idt::create_idt())) };
    let idt = unsafe { &mut *(idt::IDT.as_mut().unwrap().as_mut_ptr()) };
    // Init the interrupt descriptor table, handling cpu exceptions
    unsafe { idt.load_unsafe() };
    unsafe { hardware::PICS.lock().initialize() }; // Init pic, for hardware interrupts (Time, Keyboard...)
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
}
