use self::memory::handler::MemoryHandler;

pub mod acpi;
pub mod disk;
#[cfg(feature = "fs")]
pub mod fs;
pub mod gdt;
pub mod graphics;
pub mod interrupts;
pub mod memory;
pub mod mouse;
pub mod network;
#[cfg(feature = "pci-ids")]
pub mod pci; // pci id's Adds 2MB to kernel size !
pub mod qemu_in;
pub mod rand;
pub mod task;
pub mod terminal;
pub mod time;
pub mod userland;
pub mod video;
pub mod ps2;

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
pub enum DriverInitError {}
#[allow(clippy::type_complexity)]
pub const DRIVERS: &[(&str, fn() -> ())] = &[
    ("log", crate::user::log::initialize_logger),
    ("heap & frame allocation", super::memory::handler::init),
    ("gdt", super::gdt::init),
    ("ACPI", super::acpi::init),
    ("Ps2 Controller", super::ps2::init),
    ("interrupts", super::interrupts::init),
    ("Pci devices", super::pci::init),
    ("disks", super::disk::init),
    ("timer", super::time::init),
    ("graphics", super::video::init_graphics),
    #[cfg(feature = "fs")]
    ("filesystem (indexing disk)", fs::init),
    // ("ACPI", super::acpi::init),
    #[cfg(feature = "apic")]
    ("APIC", super::interrupts::apic::init),
    #[cfg(feature = "smp")]
    ("multiprocessing (SMP)", super::smp::init),
    // ("Userland (Ring 3)", super::userland::go_ring3),
    ("Random numbers", super::rand::init),
    ("Network", super::network::init),
    ("Mouse", super::mouse::init),
];

//TODO Specify a bit more what is a driver... Cuz rn it's just smth that needs to be initialised
pub fn init_drivers() {
    'main: for (name, init_fun) in DRIVERS.iter() {
        log::info!("Initialising {}... ", name);
        init_fun()
    }
}
