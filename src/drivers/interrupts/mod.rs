use super::Driver;

pub mod exceptions;
pub mod hardware;
pub mod idt;


pub struct InterruptsDriver {

}
impl Driver for InterruptsDriver {
    fn new() -> Self where Self: Sized {
        Self {

        }
    }

    fn name(&self) -> &str {
        "Interrupts"
    }

    fn init(&mut self) -> Result<(), super::DriverError> {
        idt::IDT.load(); // Init the interrupt descriptor table, handling cpu exceptions
        unsafe { hardware::PICS.lock().initialize() }; // Init pic, for hardware interrupts (Time, Keyboard...)
        x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
        Ok(())
    }

    fn required(&self) -> &str {
        "GDT"
    }
}