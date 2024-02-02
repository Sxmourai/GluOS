pub mod ethernet;
pub use ethernet::*;

use crate::pci_manager;

pub fn init() {
    for (loc, d) in pci_manager!().iter() {
        if d.class.id() == 2 {
            for subclass in d.class.subclasses() {
                if subclass.id() == 0 {
                    let mut drv = ethernet::E1000NetworkDriver::new(d);
                    match drv.start() {
                        Ok(_) => {
                            log::info!("Initialised a ethernet driver !");
                        },
                        Err(e) => match e {
                            E1000NetworkDriverInitError::CantReadMac => {
                                log::error!("Failed to initialise ethernet driver !");
                            },
                        },
                    }
                }
            }
        }
    }
}