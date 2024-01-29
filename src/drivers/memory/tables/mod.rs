//! Code inspired from rsdp crate

use alloc::{vec::Vec, string::{String, ToString}};

use crate::{mem_handler, boot_info};

use self::acpi::AcpiHandler;

use super::{handler::MemoryHandler, read_phys_memory_and_map};

pub mod rsdt; // pub ?
pub mod madt;
pub mod hpet;
pub mod waet;
pub mod acpi;
pub mod fadt;

static ACPI_HEAD_SIZE: usize = core::mem::size_of::<ACPISDTHeader>();


#[derive(Debug)]
#[repr(C, packed)]
pub struct ACPISDTHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oemid: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

pub struct DescriptorTablesHandler {
    pub acpi: AcpiHandler,
    pub madt: madt::MADT,
    pub hpet: &'static hpet::HPET,
    pub waet: &'static waet::WAET,
}
impl DescriptorTablesHandler {
    /// Initialises the descriptor tables handler, and makes it accessible via descriptor_tables!()
    pub fn init() {
        let physical_memory_offset = unsafe{boot_info!()}.physical_memory_offset;
        let rsdt = rsdt::search_rsdt(physical_memory_offset);
        let mut acpi = None;
        let mut madt = None;
        let mut hpet = None;
        let mut waet = None;
        for (i, ptr) in rsdt.pointer_to_other_sdt.iter().enumerate() {
            let end_page = 0xFFFFFFFF + (i * 4096) as u64;
            let (header, table_bytes) = read_sdt(*ptr as u64, end_page);

            //TODO Make parsing in another function for cleaner code
            match String::from_utf8_lossy(&header.signature).to_string().as_str() {
                "FACP" => acpi = Some(AcpiHandler::new(table_bytes)),
                "APIC" => madt = unsafe { madt::MADT::new(table_bytes) },
                "HPET" => hpet = unsafe { hpet::handle_hpet(table_bytes) },
                "WAET" => waet = unsafe { waet::handle_waet(table_bytes) },
                _ => {
                    log::error!("Couldn't parse table: {}\nRAW: {:?}\nHeader: {:?}\nPhys Address: {:x}\t-\tVirt Address: {:?}\nNumber: {}",
                    String::from_utf8_lossy(&header.signature).to_string().as_str(), 
                    table_bytes, 
                    header, 
                    ptr, 
                    table_bytes.as_ptr(), 
                    i
                );
                }
            };
        }
        unsafe { crate::state::DESCRIPTOR_TABLES.replace(Self {
            acpi: acpi.unwrap(), //TODO, handle if we don't find a table
            madt: madt.unwrap(), //TODO, handle if we don't find a table
            hpet: hpet.unwrap(), //TODO, handle if we don't find a table
            waet: waet.unwrap(), //TODO, handle if we don't find a table
        }) };
        
    }

    pub fn num_core(&self) -> usize {
        self.madt.cores.len()
    }
}
// A function because 10 lines upper we use handle_...
//TODO Maybe remove these calls (10 lines upper)

fn read_sdt(
    ptr: u64,
    end_page: u64,
) -> (&'static ACPISDTHeader, &'static [u8]) {
    let bytes = unsafe { read_phys_memory_and_map(ptr, ACPI_HEAD_SIZE, end_page) };
    let entry: &ACPISDTHeader = unsafe { &*(bytes.as_ptr() as *const _) };
    let bytes = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), entry.length as usize) };
    (entry, bytes)
}


#[repr(C)]
#[derive(Debug)]
pub struct GenericAddressStructure {
    address_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_size: u8,
    address: u64,
}
#[repr(C, packed)] //TODO Merge GenericAddressStructure and AddressStructure ?
#[derive(Debug)]
pub struct AddressStructure {
    address_space_id: u8,
    register_bit_width: u8,
    register_bit_offset: u8,
    reserved: u8,
    address: u64,
}
