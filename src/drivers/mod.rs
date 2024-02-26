use core::pin::Pin;

use alloc::{boxed::Box, format, string::ToString, vec::Vec};
use lazy_static::lazy_static;

use self::task::Task;

pub mod acpi;
pub mod disk;
#[cfg(feature = "fs")]
pub mod fs;
pub mod gdt;
pub mod interrupts;
pub mod memory;
pub mod mouse;
pub mod network;
#[cfg(feature = "pci-ids")]
pub mod pci; // pci id's Adds 2MB to kernel size !
pub mod ps2;
pub mod qemu_in;
pub mod rand;
pub mod task;
pub mod terminal;
pub mod time;
pub mod userland;
pub mod video;

fn display_driver(driver: Driver) -> alloc::string::String {
    alloc::format!(
        "Driver {{ name: {} requires: {:?} }}",
        driver.name(),
        driver.requires
    )
}

#[macro_export]
macro_rules! make_driver {
    ($name: ident, $func: expr, requires=[$($requires:ident),*]) => {{
        Driver{name:DriverId::$name, task: Task::new($func), requires: alloc::vec![$(DriverId::$requires,)*]}
    }};
    ($name: ident, $func: expr) => {{
        make_driver!($name, $func, requires=[Logger])
    }};
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Ord)]
pub enum DriverId {
    Logger,
    Heap,
    Mapper,
    Gdt,
    Acpi,
    Ps2Controller,
    Interrupts,
    Pci,
    Pit,
    Graphics,
    Disk,
    Filesystem,
}
impl DriverId {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Logger => "Logger",
            Self::Heap => "Heap",
            Self::Mapper => "Mapper",
            Self::Gdt => "Gdt",
            Self::Acpi => "Acpi",
            Self::Ps2Controller => "Ps2Controller",
            Self::Interrupts => "Interrupts",
            Self::Pci => "Pci",
            Self::Pit => "Pit",
            Self::Graphics => "Graphics",
            Self::Disk => "Disk",
            Self::Filesystem => "Filesystem",
        }
    }
}
pub struct Driver {
    pub name: DriverId,
    pub task: Task,
    // TODO Requirements in executor
    pub requires: Vec<DriverId>,
}
impl Driver {
    pub fn name(&self) -> &'static str {
        self.name.name()
    }
}
////////// TODO Remove the need to increment the len manually
////////// The best would be a vec/slice, but no vec because we don't have heap allocation and no slice because we can't use static because impl Future isn't sized !
pub fn get_drivers() -> Vec<Driver> {
    alloc::vec![
        // By default require logger, overwrite that.
        make_driver!(Logger, async { crate::user::log::init() }, requires = []),
        // make_driver!(Heap, crate::drivers::memory::init()), manually called in boot, to have executor
        make_driver!(Gdt, async { crate::drivers::gdt::init() }),
        make_driver!(
            Acpi,
            crate::drivers::acpi::init(),
            requires = [Logger, Heap, Mapper]
        ),
        make_driver!(Ps2Controller, crate::drivers::ps2::init()),
        make_driver!(
            Interrupts,
            async { crate::drivers::interrupts::init() },
            requires = [Logger, Gdt]
        ),
        make_driver!(Pci, async { crate::drivers::pci::init() }),
        make_driver!(Pit, async { crate::drivers::time::init() }),
        make_driver!(Graphics, async { crate::drivers::video::init() }),
        make_driver!(Disk, async { crate::drivers::disk::init() }),
        #[cfg(feature = "fs")]
        make_driver!(
            Filesystem,
            crate::drivers::fs::init(),
            requires = [Logger, Disk]
        ),
        // #[cfg(feature = "apic")]
        // ("APIC", super::interrupts::apic::init),
        // #[cfg(feature = "smp")]
        // ("multiprocessing (SMP)", super::smp::init),
        // ("Userland (Ring 3)", super::userland::go_ring3),
        // ("Random numbers", super::rand::init),
        // ("Network", super::network::init),
        // Don't need to init mouse driver cuz we don't have a use for it currently
        // ("Mouse", super::mouse::init),
    ]
}
// pub static DRIVERS: &'static mut [Task] = &mut [
// ];
// }

//TODO Specify a bit more what is a driver... Cuz rn it's just smth that needs to be initialised
