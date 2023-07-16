use crate::{
    gdt, interrupts,
    task::executor::Executor,
};
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

pub fn init() -> () {
    gdt::init();
    interrupts::IDT.load(); // Init the interrupt descriptor table, handling cpu exceptions
    unsafe { interrupts::PICS.lock().initialize() }; // Init pic, for hardware interrupts (Time, Keyboard...)
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
}

pub fn end() -> ! {
    let mut executor = Executor::new();
    // executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}