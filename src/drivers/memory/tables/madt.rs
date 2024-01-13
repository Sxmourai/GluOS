use alloc::vec::Vec;

use super::ACPISDTHeader;


#[repr(C, packed)]
// #[derive(Debug)] Can't derive debug because packed struct
struct RawMADT {
    h: ACPISDTHeader,
    local_apic_addr: u32,
    flags: u32,
    //Entries https://wiki.osdev.org/MADT
    //Entry Type 0: Processor Local APIC
    // ETC
}
pub struct MADT {
    #[allow(unused)]
    // Do the parsing of MADT in new
    pub inner: &'static RawMADT,
    pub fields: Vec<&'static dyn APICRecord>,
    pub num_core: Vec<(usize, u8)>, // (core id, apic id)
}
impl MADT {
    pub unsafe fn new(bytes: &[u8]) -> Self {
        let inner = unsafe { &*(bytes.as_ptr() as *const RawMADT) };
        Self {
            inner,
            fields: Vec::new(),
            num_core: Vec::new(),
        }
    }
}

trait APICRecord: core::fmt::Debug + Sync {}

#[repr(C, packed)]
#[derive(Debug)]
struct ProcLocalAPIC {
    // Entry Type 0: Processor Local APIC https://wiki.osdev.org/MADT#Entry_Type_0:_Processor_Local_APIC
    acpi_proc_id: u8,
    apic_id: u8,
    flags: u32,
}
impl APICRecord for ProcLocalAPIC {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPIC {
    // Entry Type 1: I/O APIC
    io_apic_id: u8,
    reserved: u8,
    io_apic_address: u32,
    global_system_interrupt_base: u32,
}
impl APICRecord for IOAPIC {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPICInterruptSourceOverride {
    // Entry Type 2: IO/APIC Interrupt Source Override
    bus_source: u8,
    irq_source: u8,
    global_system_interrupt: u32,
    flags: u16,
}
impl APICRecord for IOAPICInterruptSourceOverride {}

#[repr(C, packed)]
#[derive(Debug)]
struct IOAPICNonMaskableInterruptSource {
    // Entry type 3: IO/APIC Non-maskable interrupt source
    nmi_source: u8,
    reserved: u8,
    flags: u16,
    global_system_interrupt: u32,
}
impl APICRecord for IOAPICNonMaskableInterruptSource {}

#[repr(C, packed)]
#[derive(Debug)]
struct LocalAPICNonMaskableInterrupts {
    // Entry Type 4: Local APIC Non-maskable interrupts
    acpi_proc_id: u8, // (0xFF means all processors)
    flags: u16,
    lint: u16,
}
impl APICRecord for LocalAPICNonMaskableInterrupts {}

#[repr(C, packed)]
#[derive(Debug)]
struct LocalAPICAddressOverride {
    // Entry Type 5: Local APIC Address Override
    reserved: u16,
    phys_addr_local_apic: u64,
}
impl APICRecord for LocalAPICAddressOverride {}

#[repr(C, packed)]
#[derive(Debug)]
struct ProcLocalx2Apic {
    // Entry Type 9: Processor Local x2APIC
    reserved: u16,
    proc_local_x2apic_id: u32,
    flags: u32,
    acpi_id: u32,
}
impl APICRecord for ProcLocalx2Apic {}


pub unsafe fn handle_apic(bytes: &[u8]) -> Option<MADT> {
    let mut madt = unsafe { MADT::new(bytes) };

    let mut start_idx = core::mem::size_of::<RawMADT>(); // Start at size of MADT - fields
                                                       //TODO Make proper stop handling (rn it stops when there are no more bytes, but if the provided bytes is too long, kernel panic)
    let mut num_core: usize = 0;
    loop {
        if start_idx + 1 >= bytes.len() {
            break;
        } // Done looping over all records
        let record_type = &bytes[start_idx + 0];
        let record_length = &bytes[start_idx + 1];
        start_idx += match record_type {
            0 => {
                // Entry Type 0: Processor Local APIC
                let proc_local_apic =
                    unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalAPIC) };
                madt.fields.push(proc_local_apic);
                madt.num_core.push((num_core, proc_local_apic.acpi_proc_id));
                num_core += 1;
                8
            }
            1 => {
                // Entry Type 1: I/O APIC
                madt.fields
                    .push(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPIC) });
                12
            }
            2 => {
                // Entry Type 2: IO/APIC Interrupt Source Override
                madt.fields.push(unsafe {
                    &*(bytes[start_idx..].as_ptr() as *const IOAPICInterruptSourceOverride)
                });
                10
            }
            3 => {
                // Entry type 3: IO/APIC Non-maskable interrupt source
                madt.fields.push(unsafe {
                    &*(bytes[start_idx..].as_ptr() as *const IOAPICNonMaskableInterruptSource)
                });
                10
            }
            4 => {
                // Entry Type 4: Local APIC Non-maskable interrupts
                madt.fields.push(unsafe {
                    &*(bytes[start_idx..].as_ptr() as *const LocalAPICNonMaskableInterrupts)
                });
                5
            }
            5 => {
                // Entry Type 5: Local APIC Address Override
                madt.fields.push(unsafe {
                    &*(bytes[start_idx..].as_ptr() as *const LocalAPICAddressOverride)
                });
                12
            }
            9 => {
                // Entry Type 9: Processor Local x2APIC
                madt.fields
                    .push(unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalx2Apic) });
                16
            }

            _ => {
                panic!(
                    "Unrecognised record entry type: {} | length: {record_length}",
                    record_type
                )
            } //TODO Improve error handling
        }
    }
    Some(madt)
}