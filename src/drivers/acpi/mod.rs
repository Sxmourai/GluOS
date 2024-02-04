use crate::descriptor_tables;

use self::tables::DescriptorTablesHandler;

pub mod tables;

pub fn init() {
    unsafe { crate::state::DESCRIPTOR_TABLES.replace(DescriptorTablesHandler::new().unwrap()) };
}
