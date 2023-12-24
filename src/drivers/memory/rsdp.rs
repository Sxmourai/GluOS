//! Code inspired from rsdp crate

use core::{fmt::Debug, num, slice::from_raw_parts};

use alloc::{vec::Vec, format, string::{ToString, String}};
use hashbrown::HashMap;
use log::trace;
use x86_64::{structures::paging::{PageTableFlags, PhysFrame, Size4KiB, Mapper, Page}, PhysAddr, VirtAddr};


static ACPI_HEAD_SIZE:usize = core::mem::size_of::<ACPISDTHeader>();

use crate::{println, find_string, serial_println, serial_print, serial_print_all_bits, print, list_to_num, ptrlist_to_num, bytes};

use super::{read_memory, read_phys_memory_and_map, handler::MemoryHandler};
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

fn search_rsdp_in_page(page:u64, physical_memory_offset:u64) -> Option<&'static RSDPDescriptor>{
    let bytes_read = unsafe { read_memory((page+physical_memory_offset) as *const u8, 4096) };
    if let Some(offset) = find_string(&bytes_read, RSDP_SIGNATURE) {
        let sl: &[u8] = &bytes_read[offset..offset+core::mem::size_of::<RSDPDescriptor>()];
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
pub fn search_rsdp(physical_memory_offset:u64) -> &'static RSDPDescriptor {
    trace!("Searching RSDP in first memory region");
    for i in (0x80000..0x9ffff).step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(i,physical_memory_offset) {
            return rsdp;
        }
    }
    trace!("Searching RSDP in second memory region");
    for j in (0xe0000..0xfffff).step_by(4096) {
        if let Some(rsdp) = search_rsdp_in_page(j,physical_memory_offset) {
            return rsdp;
        }
    }
    panic!("Didn't find rsdp !");
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

pub struct RSDT {
    pub h: &'static ACPISDTHeader,
    pointer_to_other_sdt: Vec<u32>,
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
struct RMADT {
    h: ACPISDTHeader,
    local_apic_addr: u32,
    flags: u32,
    //Entries https://wiki.osdev.org/MADT
    //Entry Type 0: Processor Local APIC
    // ETC
}
struct MADT{
    pub inner: &'static RMADT,
    pub fields: Vec<&'static dyn APICRecord>,
    pub num_core: Vec<(usize, u8)>, // (core id, apic id)
}
impl MADT {
    pub unsafe fn new(bytes:&[u8]) -> Self {
        Self {
            inner: &*(bytes.as_ptr() as *const RMADT),
            fields: Vec::new(),
            num_core: Vec::new(),
        }
    }
}

trait APICRecord : Debug + Sync {}

#[repr(C, packed)]
#[derive(Debug)]
struct ProcLocalAPIC { // Entry Type 0: Processor Local APIC https://wiki.osdev.org/MADT#Entry_Type_0:_Processor_Local_APIC
    acpi_proc_id: u8,
    apic_id: u8,
    flags: u32,
}
impl APICRecord for ProcLocalAPIC {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPIC { // Entry Type 1: I/O APIC
    io_apic_id: u8,
    reserved: u8,
    io_apic_address: u32,
    global_system_interrupt_base: u32,
}
impl APICRecord for IOAPIC {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPICInterruptSourceOverride { // Entry Type 2: IO/APIC Interrupt Source Override
    bus_source: u8,
    irq_source: u8,
    global_system_interrupt: u32,
    flags: u16,
}
impl APICRecord for IOAPICInterruptSourceOverride {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPICNonMaskableInterruptSource{ // Entry type 3: IO/APIC Non-maskable interrupt source
    nmi_source: u8,
    reserved: u8,
    flags: u16,
    global_system_interrupt: u32,
}
impl APICRecord for IOAPICNonMaskableInterruptSource {}

#[repr(C, packed)]
#[derive(Debug)]
struct LocalAPICNonMaskableInterrupts { // Entry Type 4: Local APIC Non-maskable interrupts
    acpi_proc_id: u8, // (0xFF means all processors)
    flags: u16,
    lint: u16,
}
impl APICRecord for LocalAPICNonMaskableInterrupts {}

#[repr(C, packed)]
#[derive(Debug)]
struct LocalAPICAddressOverride { // Entry Type 5: Local APIC Address Override
    reserved: u16,
    phys_addr_local_apic: u64,
}
impl APICRecord for LocalAPICAddressOverride {}

#[repr(C, packed)]
#[derive(Debug)]
struct ProcLocalx2Apic { // Entry Type 9: Processor Local x2APIC
    reserved: u16,
    proc_local_x2apic_id: u32,
    flags: u32,
    acpi_id: u32,
}
impl APICRecord for ProcLocalx2Apic {}

#[repr(C, packed)]
pub struct HPET {
    header: ACPISDTHeader,
    hardware_rev_id: u8,
    comparator_count: u8,
    counter_size: u8,
    reserved: u8,
    legacy_replacement: u8,
    pci_vendor_id: u16,
    address: AddressStructure,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8,
}
impl Debug for HPET {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let min_tick = self.minimum_tick;
        let vendor_id = self.pci_vendor_id;
        f.debug_struct("HPET")
        .field("header", &self.header)
        .field("hardware_rev_id", &self.hardware_rev_id)
        .field("comparator_count", &self.comparator_count)
        .field("counter_size", &self.counter_size)
        .field("reserved", &self.reserved)
        .field("legacy_replacement", &self.legacy_replacement)
        .field("pci_vendor_id", &vendor_id)
        .field("address", &self.address)
        .field("hpet_number", &self.hpet_number)
        .field("minimum_tick", &min_tick)
        .field("page_protection", &self.page_protection).finish()
    }
}
#[repr(C, packed)]
struct WAET {// TODO Contribute to osdev, to make a page for this
    header: ACPISDTHeader,
    emu_dev_flags: u32,
}
impl Debug for WAET {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let dev_flags = self.emu_dev_flags;
        f.debug_struct("WAET")
        .field("header", &self.header)
        .field("emu_dev_flags", &format!("{:b}",dev_flags))
        .finish()
    }
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




fn get_rsdt(mem_handler: &mut MemoryHandler, rsdt_addr: u64) -> RSDT {
    // trace!("Getting RSDT at {}", rsdt_addr);
    let (rsdt_header, raw) = read_sdt(mem_handler, rsdt_addr, rsdt_addr);
    
    let sdts_size = (rsdt_header.length as usize - ACPI_HEAD_SIZE); // / core::mem::size_of::<u32>();
    let sdts_offset = ACPI_HEAD_SIZE;
    let ptr_addr = raw.as_ptr() as usize + sdts_offset;
    let sdts = unsafe { from_raw_parts(ptr_addr as *const u8, sdts_size) };
    let mut pointer_to_other_sdt = Vec::new();
    for i in (0..sdts.len()).step_by(4) {
        let addr = ptrlist_to_num(&mut sdts[i..i+4].into_iter());
        pointer_to_other_sdt.push(addr);
    }
    RSDT { h: rsdt_header, pointer_to_other_sdt }
}
// #[derive(Send, Sync)]
pub struct DescriptorTablesHandler {
    facp: Option<&'static FADT>,
    madt: Option<MADT>,
    hpet: Option<&'static HPET>,
    waet: Option<&'static WAET>,
}
impl DescriptorTablesHandler {
    pub fn new(mem_handler: &mut MemoryHandler, physical_memory_offset:u64) -> Self {
        let rsdp = search_rsdp(physical_memory_offset);
        let rsdt = get_rsdt(mem_handler, rsdp.rsdt_addr as u64);
        let mut _self = Self {facp:None,madt:None,hpet:None,waet:None,};

        for (i,ptr) in rsdt.pointer_to_other_sdt.iter().enumerate() {
            let end_page = 0xFFFFFFFF+(i*4096) as u64;
            let (header, table_bytes) = read_sdt(mem_handler, *ptr as u64, end_page);

            //TODO Make parsing in another function for cleaner code
            let binding = String::from_utf8_lossy(&header.signature).to_string();
            let str_table = binding.as_str();
            match str_table {
                "FACP" => _self.facp = unsafe { handle_fadt(table_bytes) },
                "APIC" => _self.madt = unsafe { handle_apic(table_bytes) },
                "HPET" => _self.hpet = unsafe { handle_hpet(table_bytes) },
                "WAET" => _self.waet = unsafe { handle_waet(table_bytes) },
                _ => {
                    panic!("Couldn't parse table: {}\nRAW: {:?}\nHeader: {:?}\nPhys Address: {:x}\t-\tVirt Address: {:?}\nNumber: {}",str_table, table_bytes, header, ptr, table_bytes.as_ptr(), i);
                },
            };
        }
        _self
    }
    pub fn num_core(&self) -> usize {self.madt.as_ref().unwrap().num_core.len()}
}

unsafe fn handle_fadt(bytes: &[u8]) -> Option<&'static FADT> {
    Some(unsafe { &*(bytes.as_ptr() as *const FADT) })
}
unsafe fn handle_apic(bytes: &[u8]) -> Option<MADT> {
    let mut madt = unsafe { MADT::new(bytes) };
    
    let mut start_idx = core::mem::size_of::<RMADT>(); // Start at size of MADT - fields
    //TODO Make proper stop handling (rn it stops when there are no more bytes, but if the provided bytes is too long, kernel panic)
    let mut num_core:usize = 0;
    loop {
        if start_idx+1 >= bytes.len() {break} // Done looping over all records
        let record_type =   &bytes[start_idx+0];
        let record_length = &bytes[start_idx+1];
        start_idx += match record_type {
            0 => { // Entry Type 0: Processor Local APIC
                let proc_local_apic = unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalAPIC) };
                madt.fields.push(proc_local_apic);
                madt.num_core.push((num_core, proc_local_apic.acpi_proc_id));
                num_core += 1;
                8
            },
            1 => { // Entry Type 1: I/O APIC
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPIC)});
                12
            },
            2 => { // Entry Type 2: IO/APIC Interrupt Source Override
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPICInterruptSourceOverride)});
                10
            },
            3 => { // Entry type 3: IO/APIC Non-maskable interrupt source
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPICNonMaskableInterruptSource)});
10
            },
            4 => { // Entry Type 4: Local APIC Non-maskable interrupts
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const LocalAPICNonMaskableInterrupts)});
                5
            },
            5 => { // Entry Type 5: Local APIC Address Override
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const LocalAPICAddressOverride)});
                12
            },
            9 => { // Entry Type 9: Processor Local x2APIC
                madt.fields.push(unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalx2Apic)});
                16
            },

            _ => {panic!("Unrecognised record entry type: {} | length: {record_length}",record_type)},//TODO Improve error handling
        }
    }
    Some(madt)
}
unsafe fn handle_hpet(bytes: &[u8]) -> Option<&'static HPET> {
    Some(unsafe { &*(bytes.as_ptr() as *const HPET) })
}
unsafe fn handle_waet(bytes: &[u8]) -> Option<&'static WAET> {
    Some(unsafe { &*(bytes.as_ptr() as *const WAET) })
}


fn read_sdt(mem_handler: &mut MemoryHandler, ptr:u64, end_page:u64) -> (&'static ACPISDTHeader, &'static [u8]) {
    let bytes = unsafe { read_phys_memory_and_map(mem_handler, ptr, ACPI_HEAD_SIZE, end_page) };
    let entry: &ACPISDTHeader = unsafe { &*(bytes.as_ptr() as *const _) };
    let bytes = unsafe { read_memory(bytes.as_ptr(), entry.length as usize) };
    (entry, bytes)
}
