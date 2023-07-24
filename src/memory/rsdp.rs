//! Code inspired from rsdp crate

use core::fmt::Debug;

use alloc::{vec::Vec, format, string::{ToString, String}};
use hashbrown::HashMap;
use x86_64::{structures::paging::{PageTableFlags, PhysFrame, Size4KiB, Mapper, Page}, PhysAddr, VirtAddr};


static ACPI_HEAD_SIZE:usize = core::mem::size_of::<ACPISDTHeader>();

use crate::{println, find_string, serial_println, serial_print, serial_print_all_bits, memory::read_memory, print};
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

fn search_rsdp_in_page(i:u64, physical_memory_offset:u64) {
    let bytes_read = unsafe { crate::memory::read_memory((i+physical_memory_offset) as *const u8, 4096) };
    if let Some(offset) = find_string(&bytes_read, RSDP_SIGNATURE) {
        serial_println!("Found RSDP pointer");
        let sl: &[u8] = &bytes_read[offset..offset+core::mem::size_of::<RSDPDescriptor>()];
        // Check that the bytes_in_memory size matches the size of RSDPDescriptor
        assert_eq!(sl.len(), core::mem::size_of::<RSDPDescriptor>());

        let rsdp_bytes: &[u8; core::mem::size_of::<RSDPDescriptor>()] =
        sl.try_into().expect("Invalid slice size");
        let rsdp_descriptor: &RSDPDescriptor = unsafe { &*(rsdp_bytes.as_ptr() as *const _) };
        
        //TODO Verify checksum

        let addr = rsdp_descriptor.rsdt_addr as usize;
        let rsdt_size = core::mem::size_of::<RSDT>();
        serial_println!("Getting RSDT at address (physical): {:x}", addr);
        let rsdt_page_bytes = unsafe { crate::memory::read_phys_memory_and_map(addr as u64, rsdt_size, 0xFFFFFFFFFFF) };
        
        let rsdt: &RSDT = unsafe { &*(rsdt_page_bytes.as_ptr() as *const _) };
        
        // let pointer_to_other_sdt_size = rsdt.h.length as usize - core::mem::size_of::<ACPISDTHeader>();
        // let n_fields = pointer_to_other_sdt_size / 4; // u32 is 4 bytes
        // let u8_sdts = &rsdt_page_bytes[ACPI_HEAD_SIZE..rsdt.h.length as usize];
        // let sdts = u8_to_u32(u8_sdts);

        for (i,ptr) in rsdt.pointer_to_other_sdt.iter().enumerate() {
            let location = *ptr as u64;
            let size = ACPI_HEAD_SIZE;
            let end_page = 0xFFFFFFFF+(i*4096) as u64;
            let bytes = unsafe { crate::memory::read_phys_memory_and_map(location, ACPI_HEAD_SIZE, end_page) };
            
            let entry: &ACPISDTHeader = unsafe { &*(bytes.as_ptr() as *const _) };
            let table = parse_table(entry, bytes.as_ptr() as usize);
            serial_println!("Parsed table: {:?}",table);
        }
    }
}


//TODO Support ACPI version 2 https://wiki.osdev.org/RSDP
pub fn search_rsdp(physical_memory_offset:u64) {
    for i in (0x80000..0x9ffff).step_by(4096) {search_rsdp_in_page(i,physical_memory_offset)}
    for j in (0xe0000..0xfffff).step_by(4096) {search_rsdp_in_page(j,physical_memory_offset)}
}


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

// trait StandartDescriptorTable {}

#[repr(C)]
pub struct RSDT {
    pub h: ACPISDTHeader,
    //TODO DONT USE THIS FIELD, USED FOR ALIGNEMENT, we need to :
    // Read once the rsdt to parse the 'length' field
    // Read it a second time, but this time read &[u8;rsdt.length]
    // And find a way to make point_to_other_sdt dynamically changed in size
    // From my testing, it never goes beyond 4...
    pointer_to_other_sdt: [u32; 4], // Placeholder for the array; its actual size will be determined at runtime.
}
impl RSDT {
    fn pointer_to_other_sdt(&self) -> &[u32] {
        let size = (self.h.length - core::mem::size_of::<ACPISDTHeader>() as u32) / core::mem::size_of::<u32>() as u32;
        let ptr = &self.pointer_to_other_sdt as *const u32;
        unsafe { core::slice::from_raw_parts(ptr, size as usize) }
    }
}

#[repr(C)]
#[derive(Debug)]
struct FADT {
    h: ACPISDTHeader,
    firmware_ctrl: u32,
    dsdt: u32,
    reserved: u8,
    preferred_power_management_profile: u8,
    sci_interrupt: u16,
    smi_command_port: u32,
    acpi_enable: u8,
    acpi_disable: u8,
    s4bios_req: u8,
    pstate_control: u8,
    pm1a_event_block: u32,
    pm1b_event_block: u32,
    pm1a_control_block: u32,
    pm1b_control_block: u32,
    pm2_control_block: u32,
    pm_timer_block: u32,
    gpe0_block: u32,
    gpe1_block: u32,
    pm1_event_length: u8,
    pm1_control_length: u8,
    pm2_control_length: u8,
    pm_timer_length: u8,
    gpe0_length: u8,
    gpe1_length: u8,
    gpe1_base: u8,
    c_state_control: u8,
    worst_c2_latency: u16,
    worst_c3_latency: u16,
    flush_size: u16,
    flush_stride: u16,
    duty_offset: u8,
    duty_width: u8,
    day_alarm: u8,
    month_alarm: u8,
    century: u8,
    boot_architecture_flags: u16,
    reserved2: u8,
    flags: u32,
    reset_reg: GenericAddressStructure,
    reset_value: u8,
    reserved3: [u8; 3],
    x_firmware_control: u64,
    x_dsdt: u64,
    x_pm1a_event_block: GenericAddressStructure,
    x_pm1b_event_block: GenericAddressStructure,
    x_pm1a_control_block: GenericAddressStructure,
    x_pm1b_control_block: GenericAddressStructure,
    x_pm2_control_block: GenericAddressStructure,
    x_pm_timer_block: GenericAddressStructure,
    x_gpe0_block: GenericAddressStructure,
    x_gpe1_block: GenericAddressStructure,
}



#[repr(C)]
#[derive(Debug)]
struct MADT {
    h: ACPISDTHeader,
    local_apic_addr: u32,
    flags: u32,
    //Entries https://wiki.osdev.org/MADT
    //Entry Type 0: Processor Local APIC
    // ETC
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

fn parse_table(header: &ACPISDTHeader, start_address: usize) -> &'static dyn Debug {
    let raw_table = unsafe { read_memory(start_address as *const u8, header.length as usize) };
    let binding = String::from_utf8_lossy(&header.signature).to_string();
    let str_table = binding.as_str();
    let table:&'static dyn Debug = match str_table {
        "FACP" => unsafe { &*(raw_table.as_ptr() as *const FADT) },
        "APIC" => unsafe { &*(raw_table.as_ptr() as *const MADT) },
        _ => {
            panic!("Couldn't parse table: {}",str_table);
            // panic!("Couldn't parse table: {}\nRAW: {:?}",str_table, raw_table);
        },
    };
    table
}





fn u8_to_u32(u8_data: &[u8]) -> Vec<u32> {
    let mut u32_data = Vec::new();

    for i in (0..u8_data.len()).step_by(4) {
        let mut sum = 0;
        for &byte in &u8_data[i..i + 4] {
            // Perform the conversion by combining four consecutive u8 values into a u32
            sum = (sum << 8) | u32::from(byte);
        }
        u32_data.push(sum);
    }

    u32_data
}