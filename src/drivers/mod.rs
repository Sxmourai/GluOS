use core::fmt::Display;

use alloc::{vec::Vec, string::String, vec};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::error;
use spin::{Mutex, MutexGuard};

mod disk;
mod graphics;
mod interrupts;
mod memory;
mod vga;
mod id;

//HAS TO BE SORTED FOR NOW
lazy_static!{static ref DRIVERS: HashMap<DriverId, Mutex<&'static mut dyn Driver>> = HashMap::new();}

trait Driver: Sync + Send {
    fn name(&self) -> String;
    // fn version
    fn init(&mut self) -> Result<(), DriverError>;
    fn require(&self) -> Vec<DriverId>;
    fn id(&self) -> DriverId;
}
impl Display for dyn Driver {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!("Driver {{ name: {} requires: {:?} id: {} }}", self.name(), self.require(), self.id()))
    }
}
type DriverId = usize;
trait DriverIdTrait {
    fn get_driver(&self) -> Option<MutexGuard<'static, &'static mut dyn Driver>>;
}
impl DriverIdTrait for DriverId {
    fn get_driver(&self) -> Option<MutexGuard<'static, &'static mut dyn Driver>> {
        if let Some(driver) = DRIVERS.get(self) {
            Some(driver.lock())
        } else {
            None
        }
    }
}
enum DriverError {

}

//Initialises drivers in the right order 
//Tries to handle errors
pub fn init() {
    let mut initialised: Vec<DriverId> = Vec::new();
    'main: for (i, (driver_id, driver)) in DRIVERS.iter().enumerate() {
        'require: for required_dvr in driver.lock().require() {
            if !initialised.contains(&required_dvr) {
                error!("Couldn't load driver: {} because {} is not initialised !", driver.lock(), required_dvr.get_driver().unwrap());
                break 'main;
            }
        }
    }
}