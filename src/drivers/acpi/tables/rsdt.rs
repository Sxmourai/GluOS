use core::ptr::slice_from_raw_parts;

use alloc::{collections::btree_map::Range, vec::Vec};
use log::trace;

use crate::{
    acpi::tables::{read_sdt, ACPI_HEAD_SIZE},
    bit_manipulation::any_as_u8_slice,
    dbg,
    memory::handler::MemoryHandler,
};

use super::{ACPISDTHeader, SystemDescriptionPtr, SystemDescriptionTable};

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
#[must_use] pub fn find_string(bytes: &[u8], search_string: &[u8]) -> Option<usize> {
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
}
#[repr(C, packed)]
pub struct XSDPDescriptor {
    pub rsdt: RSDPDescriptor,
    pub len: u32,
    pub xsdt_addr: u64,
    pub ext_chcksum: u8,
    reserved: [u8; 3],
}

fn search_rsdp_in_page(page: u64, physical_memory_offset: u64) -> Option<&'static RSDPDescriptor> {
    let bytes_read =
        unsafe { core::slice::from_raw_parts((page + physical_memory_offset) as *const u8, 4096) };
    if let Some(offset) = find_string(bytes_read, RSDP_SIGNATURE) {
        let sl = &bytes_read[offset..offset + core::mem::size_of::<RSDPDescriptor>()];

        let rsdp_bytes: &[u8; core::mem::size_of::<RSDPDescriptor>()] =
            sl.try_into().expect("Invalid slice size");
        let rsdp_descriptor = unsafe { &*rsdp_bytes.as_ptr().cast() };

        return Some(rsdp_descriptor);
    }
    None
}

#[must_use] pub fn search_rsdp(physical_memory_offset: u64) -> &'static RSDPDescriptor {
    // chains aren't const
    let rsdp_addresses = (1_003_520..1_003_520 + 4096)
        .chain(0x80000..0x9ffff)
        .chain(0xe0000..0xfffff);
    trace!("Searching RSDP in first&second memory region");
    for i in rsdp_addresses.step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(i, physical_memory_offset) {
            return rsdp;
        }
    }
    panic!("Didn't find rsdp !");
}
///
fn get_table_ptrs<
    T: Sized
        + core::ops::Shl<usize>
        + core::ops::BitOr<Output = T>
        + core::convert::From<<T as core::ops::Shl<usize>>::Output>
        + core::default::Default
        + core::convert::From<u8>,
>(
    sdt_and_ptrs: &[u8],
    len: usize,
) -> Vec<T> {
    let mut ptrs = Vec::new();
    for i in (0..len).step_by(core::mem::size_of::<T>()) {
        ptrs.push(crate::bit_manipulation::ptrlist_to_num(
            &mut sdt_and_ptrs[i..i + core::mem::size_of::<T>()].iter(),
        ));
    }
    ptrs
}

#[must_use] pub fn get_rsdt(sdp: &SystemDescriptionPtr) -> Option<SystemDescriptionTable> {
    trace!("Getting system description table at {}", sdp.addr());
    let (sdt_header, raw) = read_sdt(sdp.addr(), sdp.addr());

    let sdts_size = sdt_header.length as usize - ACPI_HEAD_SIZE; // / core::mem::size_of::<u32>();
    let sdts_offset = ACPI_HEAD_SIZE;
    let ptr_addr = raw.as_ptr() as usize + sdts_offset;
    let sdts = unsafe { core::slice::from_raw_parts(ptr_addr as *const u8, sdts_size) };

    let sdt = match sdp {
        SystemDescriptionPtr::Root(rsdp) => {
            SystemDescriptionTable::Root((sdt_header, get_table_ptrs(sdts, sdts.len())))
        }
        SystemDescriptionPtr::Extended(xsdp) => {
            SystemDescriptionTable::Extended((sdt_header, get_table_ptrs(sdts, sdts.len())))
        }
    };
    // RSDT Checksum
    let table_bytes = any_as_u8_slice(&sdts);
    let mut sum: u8 = 0;
    for byte in table_bytes {
        sum = sum.wrapping_add(*byte);
    }
    if sum == 0 {
        log::error!("Failed doing checksum of RSDT");
        return None;
    }
    Some(sdt)
}
