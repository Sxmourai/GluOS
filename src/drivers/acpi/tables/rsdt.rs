use core::ptr::slice_from_raw_parts;

use alloc::{collections::btree_map::Range, vec::Vec};
use log::trace;

use crate::{
    acpi::tables::{read_sdt, ACPI_HEAD_SIZE}, bit_manipulation::any_as_u8_slice, dbg, memory::handler::MemoryHandler
};

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

    (0..(bytes.len() - search_len + 1)).find(|&i| &bytes[i..(i + search_len)] == search_string)
}

pub const RSDP_SIGNATURE: &[u8; 8] = b"RSD PTR ";

#[derive(Debug)]
#[repr(C, packed)]
pub struct RSDPDescriptor {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_addr: u32,
    // ! XSDT
    len: u32,
    pub xsdt_addr: u64,
    ext_chcksum: u8,
    reserved: [u8; 3],
}

fn search_rsdp_in_page(page: u64, physical_memory_offset: u64) -> Option<&'static RSDPDescriptor> {
    let bytes_read =
        unsafe { core::slice::from_raw_parts((page + physical_memory_offset) as *const u8, 4096) };
    if let Some(offset) = find_string(bytes_read, RSDP_SIGNATURE) {
        let sl = &bytes_read[offset..offset + core::mem::size_of::<RSDPDescriptor>()];
 
        let rsdp_bytes: &[u8; core::mem::size_of::<RSDPDescriptor>()] = sl.try_into().expect("Invalid slice size");
        let rsdp_descriptor = unsafe { &*(rsdp_bytes.as_ptr() as *const _) };

        return Some(rsdp_descriptor);
    }
    None
}

//TODO Support ACPI version 2 https://wiki.osdev.org/RSDP
pub fn search_rsdp(physical_memory_offset: u64) -> &'static RSDPDescriptor {
    // chains aren't const
    let rsdp_addresses = (1003520..1003520+4096).chain(0x80000..0x9ffff).chain(0xe0000..0xfffff);
    trace!("Searching RSDP in first&second memory region");
    for i in rsdp_addresses.step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(i, physical_memory_offset) {
            return rsdp;
        }
    }
    panic!("Didn't find rsdp !");
}

pub struct RSDT {
    pub header: &'static ACPISDTHeader,
    pub pointer_to_other_sdt: Vec<u32>,
}

pub fn get_rsdt(rsdt_addr: u64) -> Option<RSDT> {
    trace!("Getting RSDT at {}", rsdt_addr);
    let (rsdt_header, raw) = read_sdt(rsdt_addr, rsdt_addr);

    let sdts_size = rsdt_header.length as usize - ACPI_HEAD_SIZE; // / core::mem::size_of::<u32>();
    let sdts_offset = ACPI_HEAD_SIZE;
    let ptr_addr = raw.as_ptr() as usize + sdts_offset;
    let sdts = unsafe { core::slice::from_raw_parts(ptr_addr as *const u8, sdts_size) };
    let mut pointer_to_other_sdt = Vec::new();
    for i in (0..sdts.len()).step_by(4) {
        let addr = crate::bit_manipulation::ptrlist_to_num(&mut sdts[i..i + 4].iter());
        pointer_to_other_sdt.push(addr);
    }
    let rsdt = RSDT {
        header: rsdt_header,
        pointer_to_other_sdt,
    };
    // RSDT Checksum
    let table_bytes = any_as_u8_slice(rsdt_header);
    let mut sum: u8 = 0;
    for byte in table_bytes {
        sum = sum.wrapping_add(*byte);
    }
    if sum == 0 {
        log::error!("Failed doing checksum of RSDT");
        return None;
    }
    Some(rsdt)
}