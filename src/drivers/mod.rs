use core::{cell::Cell, fmt::Display};

use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use hashbrown::HashMap;
use lazy_static::lazy_static;
use log::{debug, error};
use spin::{Mutex, MutexGuard};

pub mod disk;
pub mod fs;
pub mod gdt;
pub mod graphics;
pub mod id;
pub mod interrupts;
pub mod memory;
pub mod task;
pub mod terminal;
pub mod time;
pub mod video;
pub mod pci;

// pub struct DriverManager {
//     pub drivers: HashMap<String, Box<dyn Driver>>,
//     pub fs_driver: Mutex<FsDriver>,
// }
// impl DriverManager {
//     pub fn new() -> Self {
//         let mut memory_driver = MemoryDriver::new();
//         memory_driver.init();
//         Self {
//             drivers: HashMap::new(),
//             fs_driver: Mutex::new(FsDriver::new()),
//         }
//     }
//     pub fn add(&mut self, driver: Box<dyn Driver>) {
//         self.drivers.insert(driver.name().to_string(), driver);
//     }
//     pub fn get_mut(&mut self, driver_name: &str) -> Option<&mut Box<dyn Driver>> {
//         return self.drivers.get_mut(driver_name);
//     }
// }
// pub fn get_driver(name: &str) -> Option<&Box<dyn Driver>> {
//     get_driver_manager().get_mut().drivers.get(name)
// }
// pub fn get_mut_driver(name: &str) -> Option<&mut Box<dyn Driver>> {
//     get_driver_manager().get_mut().drivers.get_mut(name)
// }

// pub trait Driver: Sync + Send {
//     fn new() -> Self
//     where
//         Self: Sized;
//     fn name(&self) -> &'static str;
//     // fn version
//     fn init(&mut self) -> Result<(), DriverError>;
//     fn required(&self) -> &str;
// }
// impl Display for dyn Driver {
//     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
//         f.write_fmt(format_args!(
//             "Driver {{ name: {} requires: {:?} }}",
//             self.name(),
//             self.required()
//         ))
//     }
// }
// pub trait SetGet: Driver {
//     fn set<T>(&mut self, name: &str, new_val: T);
//     fn get<T>(&mut self, name: &str) -> T;
// }
// #[derive(Debug)]
// pub enum DriverError {
//     AlreadyExists,
// }

// //Initialises drivers in the right order
// //Tries to handle errors
// pub fn init() {
//     DriverManager::new();
// }

//
// 'main: for (i, (mut driver)) in drivers.into_iter().enumerate() {
//     let driver_id = driver.name();
//     serial_print!("- {} {:?}... ", driver_id, driver.required());
//     'require: for required_dvr_id in driver.required().split(" && ") {
//         if required_dvr_id.is_empty() {
//             continue;
//         }
//         if !initialised.contains(&required_dvr_id) {
//             serial_println!(
//                 "Couldn't load driver: {} because {} is not initialised !",
//                 driver,
//                 required_dvr_id
//             );
//             break 'main;
//         }
//     }
//     let res = driver.init();
//     if res.is_ok() {
//         serial_println!("Ok");
//         initialised.push(driver_id);
//     } else {
//         serial_println!(
//             "Couldn't load driver: {}. Error: {:?} !",
//             driver,
//             res.unwrap_err()
//         );
//     }
// }
