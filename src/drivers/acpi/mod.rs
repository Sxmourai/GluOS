use crate::descriptor_tables;

use self::tables::DescriptorTablesHandler;

pub mod tables;

/// Reads low memory BIOS for RSDP, then gets RSDT, then parses the different memory tables like FADT, MADT
/// They are required for multiprocessing, checking if there is a ps2 controller (see src/drivers/ps2.rs)
/// And many more (like computer shutdown which isn't implemented rn)
pub fn init() {
    unsafe { crate::state::DESCRIPTOR_TABLES.replace(DescriptorTablesHandler::new().unwrap()) };
}
