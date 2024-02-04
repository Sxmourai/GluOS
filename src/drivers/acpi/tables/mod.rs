//! Code inspired from rsdp crate

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{boot_info, mem_handler, memory::read_phys_memory_and_map};

pub mod dsdt;
pub mod fadt;
pub mod hpet;
pub mod madt;
pub mod rsdt;
pub mod ssdt;
pub mod waet;

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
    pub fadt: &'static fadt::FADT,
    pub madt: madt::MADT,
    pub hpet: &'static hpet::HPET,
    pub waet: &'static waet::WAET,
}
impl DescriptorTablesHandler {
    pub fn new() -> Option<Self> {
        let physical_memory_offset = unsafe { boot_info!() }.physical_memory_offset;
        let rsdt = rsdt::search_rsdt(physical_memory_offset)?;
        let mut acpi = None;
        let mut madt = None;
        let mut hpet = None;
        let mut waet = None;
        for (i, ptr) in rsdt.pointer_to_other_sdt.iter().enumerate() {
            let end_page = 0xFFFFFFFF + (i * 4096) as u64;
            let (header, table_bytes) = read_sdt(*ptr as u64, end_page);

            //TODO Make parsing in another function for cleaner code
            match String::from_utf8_lossy(&header.signature)
                .to_string()
                .as_str()
            {
                "FACP" => acpi = Some(fadt::FADT::new(table_bytes)),
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
        Some(Self {
            fadt: acpi.unwrap(),
            madt: madt.unwrap(),
            hpet: hpet.unwrap(),
            waet: waet.unwrap(),
        })
    }

    pub fn num_core(&self) -> usize {
        self.madt.cores.len()
    }
}
// A function because 10 lines upper we use handle_...
//TODO Maybe remove these calls (10 lines upper)

fn read_sdt(ptr: u64, end_page: u64) -> (&'static ACPISDTHeader, &'static [u8]) {
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
