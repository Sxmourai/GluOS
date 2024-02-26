pub mod ethernet;
pub use ethernet::*;
pub mod e1000;
pub use e1000::*;

use crate::pci_manager;

pub fn init() {
    for (loc, d) in pci_manager!().iter() {
        if d.class.id() == 2 {
            for subclass in d.class.subclasses() {
                if subclass.id() == 0 {
                    let mut drv = E1000NetworkDriver::new(d);
                    match drv.start() {
                        Ok(_) => {
                            log::trace!("Initialised a ethernet driver !");
                        }
                        Err(e) => match e {
                            E1000NetworkDriverInitError::CantReadMac => {
                                log::error!("Failed to initialise ethernet driver !");
                            }
                        },
                    }
                }
            }
        }
    }
}

extern "x86-interrupt" fn handle_receive(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    crate::dbg!("Network", _stack_frame);
}
