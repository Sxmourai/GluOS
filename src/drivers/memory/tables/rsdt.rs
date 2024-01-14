use alloc::vec::Vec;
use log::trace;

use crate::memory::{handler::MemoryHandler, tables::{read_sdt, ACPI_HEAD_SIZE}};

use super::ACPISDTHeader;

/// This (usually!) contains the base address of the EBDA (Extended Bios Data Area), shifted right by 4
// const EBDA_START_SEGMENT_PTR: usize = 0x40e; // Base address in in 2 bytes
/// The earliest (lowest) memory address an EBDA (Extended Bios Data Area) can start
// const EBDA_EARLIEST_START: usize = 0x80000;
/// The end of the EBDA (Extended Bios Data Area)
// const EBDA_END: usize = 0x9ffff;
/// The start of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
// const RSDP_BIOS_AREA_START: usize = 0xe0000;
/// The end of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
// const RSDP_BIOS_AREA_END: usize = 0xfffff;

//TODO Do we really need this function ? If so maybe in utils or smth
pub fn find_string(bytes: &[u8], search_string: &[u8]) -> Option<usize> {
    let search_len = search_string.len();

    for i in 0..(bytes.len() - search_len + 1) {
        if &bytes[i..(i + search_len)] == search_string {
            return Some(i);
        }
    }

    None
}

pub const RSDP_SIGNATURE: &'static [u8; 8] = b"RSD PTR ";

#[repr(C, packed)]
pub struct RSDPDescriptor {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}

fn search_rsdp_in_page(page: u64, physical_memory_offset: u64) -> Option<&'static RSDPDescriptor> {
    let bytes_read = unsafe { 
        core::slice::from_raw_parts((page + physical_memory_offset) as *const u8, 4096)
    };
    if let Some(offset) = find_string(bytes_read, RSDP_SIGNATURE) {
        let sl: &[u8] = &bytes_read[offset..offset + core::mem::size_of::<RSDPDescriptor>()];
        // Check that the bytes_in_memory size matches the size of RSDPDescriptor
        assert_eq!(sl.len(), core::mem::size_of::<RSDPDescriptor>());

        let rsdp_bytes: &[u8; core::mem::size_of::<RSDPDescriptor>()] =
            sl.try_into().expect("Invalid slice size");
        let rsdp_descriptor: &RSDPDescriptor = unsafe { &*(rsdp_bytes.as_ptr() as *const _) };

        //TODO Verify checksum
        return Some(rsdp_descriptor);
    }
    None
}

//TODO Support ACPI version 2 https://wiki.osdev.org/RSDP
pub fn search_rsdp(physical_memory_offset: u64) -> &'static RSDPDescriptor {
    trace!("Searching RSDP in first memory region");
    for i in (0x80000..0x9ffff).step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(i, physical_memory_offset) {
            return rsdp;
        }
    }
    trace!("Searching RSDP in second memory region");
    for j in (0xe0000..0xfffff).step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(j, physical_memory_offset) {
            return rsdp;
        }
    }
    panic!("Didn't find rsdp !");
}

pub struct RSDT {
    pub header: &'static ACPISDTHeader,
    pub pointer_to_other_sdt: Vec<u32>,
}


fn get_rsdt(mem_handler: &mut MemoryHandler, rsdt_addr: u64) -> RSDT {
    trace!("Getting RSDT at {}", rsdt_addr);
    let (rsdt_header, raw) = read_sdt(mem_handler, rsdt_addr, rsdt_addr);

    let sdts_size = rsdt_header.length as usize - ACPI_HEAD_SIZE; // / core::mem::size_of::<u32>();
    let sdts_offset = ACPI_HEAD_SIZE;
    let ptr_addr = raw.as_ptr() as usize + sdts_offset;
    let sdts = unsafe { core::slice::from_raw_parts(ptr_addr as *const u8, sdts_size) };
    let mut pointer_to_other_sdt = Vec::new();
    for i in (0..sdts.len()).step_by(4) {
        let addr = crate::bit_manipulation::ptrlist_to_num(&mut sdts[i..i + 4].into_iter());
        pointer_to_other_sdt.push(addr);
    }
    RSDT {
        header: rsdt_header,
        pointer_to_other_sdt,
    }
}
pub fn search_rsdt(mem_handler: &mut MemoryHandler, physical_memory_offset: u64) -> RSDT {
    get_rsdt(mem_handler, search_rsdp(physical_memory_offset).rsdt_addr.into())
}