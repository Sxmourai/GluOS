//! Code inspired from rsdp crate

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{boot_info, dbg, mem_handler, memory::read_phys_memory_and_map};

use self::rsdt::{RSDPDescriptor, XSDPDescriptor};

pub mod dsdt;
pub mod fadt;
pub mod hpet;
pub mod madt;
pub mod rsdt;
pub mod ssdt;
pub mod waet;

static ACPI_HEAD_SIZE: usize = core::mem::size_of::<ACPISDTHeader>();

pub enum AcpiVersion {
    One,
    Two,
}

#[derive(Clone)]
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
impl core::fmt::Debug for ACPISDTHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let length = self.length;
        let oem_revision = self.oem_revision;
        let creator_id = self.creator_id;
        let creator_revision = self.creator_revision;
        f.debug_struct("ACPISDTHeader")
            .field("signature", &self.signature.map(|c| c as char))
            .field("length", &length)
            .field("revision", &self.revision)
            .field("checksum", &self.checksum)
            .field("oemid", &self.oemid.map(|c| c as char))
            .field("oem_table_id", &self.oem_table_id.map(|c| c as char))
            .field("oem_revision", &oem_revision)
            .field("creator_id", &creator_id)
            .field("creator_revision", &creator_revision)
            .finish()
    }
}

pub enum SystemDescriptionPtr {
    Root(&'static RSDPDescriptor),
    Extended(&'static XSDPDescriptor),
}
impl SystemDescriptionPtr {
    #[must_use] pub fn addr(&self) -> u64 {
        match self {
            SystemDescriptionPtr::Root(rsdp) => rsdp.rsdt_addr.into(),
            SystemDescriptionPtr::Extended(xsdp) => xsdp.xsdt_addr,
        }
    }
}

pub enum SystemDescriptionTable {
    Root((&'static ACPISDTHeader, Vec<u32>)),
    Extended((&'static ACPISDTHeader, Vec<u64>)),
}
impl SystemDescriptionTable {
    #[must_use] pub fn tables(&self) -> Vec<u64> {
        match self {
            SystemDescriptionTable::Root(rsdt) => rsdt.1.iter().map(|ptr| u64::from(*ptr)).collect(),
            SystemDescriptionTable::Extended(xsdt) => xsdt.1.clone(),
        }
    }
}

pub struct DescriptorTablesHandler {
    pub fadt: &'static fadt::FADT,
    pub madt: madt::MADT,
    pub hpet: &'static hpet::HPET,
    pub waet: &'static waet::WAET,
    pub description_table: SystemDescriptionTable,
}
impl DescriptorTablesHandler {
    pub async fn new() -> Option<Self> {
        let physical_memory_offset = unsafe { boot_info!() }.physical_memory_offset;
        let rsdp = rsdt::search_rsdp(physical_memory_offset);
        let sys_desc_ptr = if rsdp.revision == 0 {
            log::trace!("Found ACPI version 1.0");
            SystemDescriptionPtr::Root(rsdp)
        } else if rsdp.revision == 2 {
            log::trace!("Found ACPI version 2.0-6.1");
            SystemDescriptionPtr::Extended(unsafe { &*(core::ptr::addr_of!(rsdp) as *const _) })
        } else {
            log::error!("Unknown ACPI version: {}", rsdp.revision);
            return None;
        };
        let sdt = rsdt::get_rsdt(&sys_desc_ptr)?;

        let mut acpi = None;
        let mut madt = None;
        let mut hpet = None;
        let mut waet = None;
        for (i, ptr) in sdt.tables().iter().enumerate() {
            let end_page = 0xFFFF_FFFF + (i * 4096) as u64;
            let (header, table_bytes) = read_sdt(*ptr, end_page);

            match String::from_utf8_lossy(&header.signature)
                .to_string()
                .as_str()
            {
                "FACP" => acpi = { Some(fadt::FADT::new(table_bytes).await) },
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
            fadt: acpi?, // TODO Try to continue even if one table wasn't found
            madt: madt?,
            hpet: hpet?,
            waet: waet?,
            description_table: sdt,
        })
    }

    #[must_use] pub fn num_core(&self) -> usize {
        self.madt.cores.len()
    }
    #[must_use] pub fn version(&self) -> AcpiVersion {
        match self.description_table {
            SystemDescriptionTable::Root(_) => AcpiVersion::One,
            SystemDescriptionTable::Extended(_) => AcpiVersion::Two,
        }
    }
}
// impl crate::Driver for DescriptorTablesHandler {
//     fn name(&self) -> &'static str {
//         "ACPI"
//     }
//     fn init(&mut self) -> crate::task::Task {
//         Task::new(async {

//         })
//     }
//     fn required(&self) -> &str {
//         "memory"
//     }
// }

fn read_sdt(ptr: u64, end_page: u64) -> (&'static ACPISDTHeader, &'static [u8]) {
    let bytes = unsafe { read_phys_memory_and_map(ptr, ACPI_HEAD_SIZE, end_page) };
    let entry: &ACPISDTHeader = unsafe { &*bytes.as_ptr().cast() };
    let bytes = unsafe { core::slice::from_raw_parts(bytes.as_ptr(), entry.length as usize) };
    (entry, bytes)
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct GenericAddressStructure {
    address_space: u8,
    bit_width: u8,
    bit_offset: u8,
    access_size: u8,
    address: u64,
}
