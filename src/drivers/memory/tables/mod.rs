//! Code inspired from rsdp crate

use alloc::{vec::Vec, string::{String, ToString}};

use super::{handler::MemoryHandler, read_phys_memory_and_map};

mod rsdt; // pub ?
mod madt;
mod hpet;
mod waet;
mod fadt;

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
    facp: Option<&'static fadt::FADT>,
    madt: Option<madt::MADT>,
    hpet: Option<&'static hpet::HPET>,
    waet: Option<&'static waet::WAET>,
}
impl DescriptorTablesHandler {
    pub fn new(mem_handler: &mut MemoryHandler, physical_memory_offset: u64) -> Self {
        let rsdt = rsdt::search_rsdt(mem_handler, physical_memory_offset);
        let mut _self = Self {
            facp: None,
            madt: None,
            hpet: None,
            waet: None,
        };

        for (i, ptr) in rsdt.pointer_to_other_sdt.iter().enumerate() {
            let end_page = 0xFFFFFFFF + (i * 4096) as u64;
            let (header, table_bytes) = read_sdt(mem_handler, *ptr as u64, end_page);

            //TODO Make parsing in another function for cleaner code
            match String::from_utf8_lossy(&header.signature).to_string().as_str() {
                "FACP" => _self.facp = unsafe { fadt::handle_fadt(table_bytes) },
                "APIC" => _self.madt = unsafe { madt::handle_apic(table_bytes) },
                "HPET" => _self.hpet = unsafe { hpet::handle_hpet(table_bytes) },
                "WAET" => _self.waet = unsafe { waet::handle_waet(table_bytes) },
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
        _self
    }
    pub fn num_core(&self) -> usize {
        self.madt.as_ref().unwrap().num_core.len()
    }
}
// A function because 10 lines upper we use handle_...
//TODO Maybe remove these calls (10 lines upper)

fn read_sdt(
    mem_handler: &mut MemoryHandler,
    ptr: u64,
    end_page: u64,
) -> (&'static ACPISDTHeader, &'static [u8]) {
    let bytes = unsafe { read_phys_memory_and_map(mem_handler, ptr, ACPI_HEAD_SIZE, end_page) };
    let entry: &ACPISDTHeader = unsafe { &*(bytes.as_ptr() as *const _) };
    let bytes = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), entry.length as usize) };
    (entry, bytes)
}


#[repr(C)]
#[derive(Debug)]
struct GenericAddressStructure {
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
