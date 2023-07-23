//! Code inspired from rsdp crate

use crate::{println, find_string};
/// This (usually!) contains the base address of the EBDA (Extended Bios Data Area), shifted right by 4
const EBDA_START_SEGMENT_PTR: usize = 0x40e; // Base address in in 2 bytes
/// The earliest (lowest) memory address an EBDA (Extended Bios Data Area) can start
const EBDA_EARLIEST_START: usize = 0x80000;
/// The end of the EBDA (Extended Bios Data Area)
const EBDA_END: usize = 0x9ffff;
/// The start of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
const RSDP_BIOS_AREA_START: usize = 0xe0000;
/// The end of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
const RSDP_BIOS_AREA_END: usize = 0xfffff;
// Root System Description Pointer signature
pub const RSDP_SIGNATURE: &'static [u8; 8] = b"RSD PTR ";
#[repr(C, packed)]
pub struct RSDPDescriptor {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_addr: u32,
}
pub fn search_rsdp(physical_memory_offset:u64) {
    let s = |i| {
        let bytes_read = unsafe { crate::memory::read_memory((i+physical_memory_offset) as *const u8, 4096) };
        if let Some(offset) = find_string(&bytes_read, RSDP_SIGNATURE) {
            let sl: &[u8] = &bytes_read[offset..offset+core::mem::size_of::<RSDPDescriptor>()];
            // Check that the bytes_in_memory size matches the size of RSDPDescriptor
            assert_eq!(sl.len(), core::mem::size_of::<RSDPDescriptor>());

            let rsdp_bytes: &[u8; core::mem::size_of::<RSDPDescriptor>()] =
            sl.try_into().expect("Invalid slice size");

            // Reinterpret the bytes as a reference to RSDPDescriptor
            let rsdp_descriptor: &RSDPDescriptor = unsafe { &*(rsdp_bytes.as_ptr() as *const _) };
    

            // Now you have the RSDPDescriptor struct created from the bytes_in_memory
            // You can access its fields like this:
            let addr = rsdp_descriptor.rsdt_addr;
            println!("Signature: {:?}", &rsdp_descriptor.signature);
            println!("Checksum: {}", rsdp_descriptor.checksum);
            println!("OEMID: {:?}", &rsdp_descriptor.oemid);
            println!("Revision: {}", rsdp_descriptor.revision);
            println!("RsdtAddress: {:#x}", addr);
        }
    };

    for i in (0x80000..0x9ffff).step_by(4096) {s(i)}
    for j in (0xe0000..0xfffff).step_by(4096) {s(j)}
}