use self::memory::handler::MemoryHandler;

pub mod disk;
pub mod fs;
pub mod gdt;
pub mod graphics;
pub mod interrupts;
pub mod memory;
pub mod pci;// pci id's Adds 2MB to kernel size !
pub mod task;
pub mod terminal;
pub mod time;
pub mod video; 

pub trait Driver: Sync + Send {
    fn new() -> Self
    where
        Self: Sized;
    fn name(&self) -> &'static str;
    // fn version ?
    fn init(&mut self) -> Result<(), DriverInitError>;
    fn required(&self) -> &str;
}
impl core::fmt::Display for dyn Driver {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_fmt(format_args!(
            "Driver {{ name: {} requires: {:?} }}",
            self.name(),
            self.required()
        ))
    }
}
pub enum DriverInitError {

}

pub const DRIVERS: &[(&'static str, fn() -> ())] = &[
    ("log", crate::user::log::initialize_logger),
    ("heap & frame allocation", super::memory::handler::init),
    ("gdt", super::gdt::init),
    ("interrupts", super::interrupts::init),
    ("disks", super::disk::ata::init),
    ("timer", super::time::init),
    ("graphics", super::video::init_graphics),
    ("filesystem (indexing disk)", fs::init),
    ("descriptor tables", super::memory::tables::DescriptorTablesHandler::init),
    ("APIC", || unsafe { super::interrupts::apic::init().expect("Failed to init apic"); }),
];

//TODO Specify a bit more what is a driver... Cuz rn it's just smth that needs to be initialised
pub fn init_drivers() {
    'main: for (name, init_fun) in DRIVERS.into_iter() {
        log::info!("Initialising {}... ", name);
        init_fun()
    }
}
