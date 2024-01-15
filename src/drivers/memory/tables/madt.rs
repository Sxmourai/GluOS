use alloc::vec::Vec;
use log::debug;

use crate::serial_println;

use super::ACPISDTHeader;

pub type Core = (usize, u8);

#[repr(C, packed)]
pub struct RawMADT {
    pub h: ACPISDTHeader, // h.signature == [65, 80, 73, 67] == APIC
    pub local_apic_addr: u32,
    pub flags: u32,
    //Entries https://wiki.osdev.org/MADT
    //Entry Type 0: Processor Local APIC
    // ETC
}
impl core::fmt::Debug for RawMADT {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let local_apic_addr = self.local_apic_addr;
        let flags = self.flags;
        f.debug_struct("RawMADT").field("h", &self.h).field("local_apic_addr", &local_apic_addr).field("flags", &flags).finish()
    }
}
#[derive(Debug)]
pub struct MADT {
    pub inner: &'static RawMADT,
    pub fields: Vec<&'static dyn APICRecord>,
    pub cores: Vec<Core>, // (core id, apic id)
}
impl MADT {
    pub unsafe fn new(bytes: &[u8]) -> Option<Self> {
        let inner = unsafe { &*(bytes.as_ptr() as *const RawMADT) };
        let mut fields: Vec<&'static dyn APICRecord> = Vec::new();
        let mut cores = Vec::new();
        let mut start_idx = core::mem::size_of::<RawMADT>(); // Fields start at end of RawMADT
        let mut num_core = 0;
        loop {
            if start_idx + 1 >= bytes.len() {// Done looping over all records
                break;
            } 
            let record_type = &bytes[start_idx + 0];
            let record_length = &bytes[start_idx + 1];
            start_idx += match record_type {
                0 => {// Entry Type 0: Processor Local APIC
                    let proc_local_apic =
                        unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalAPIC) };
                    fields.push(proc_local_apic);
                    cores.push((num_core, proc_local_apic.acpi_proc_id));
                    num_core += 1;
                    8
                }
                1 => {// Entry Type 1: I/O APIC
                    fields
                        .push(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPIC) });
                    12
                }
                2 => {// Entry Type 2: IO/APIC Interrupt Source Override
                    fields.push(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const IOAPICInterruptSourceOverride)
                    });
                    10
                }
                3 => {// Entry type 3: IO/APIC Non-maskable interrupt source
                    fields.push(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const IOAPICNonMaskableInterruptSource)
                    });
                    10
                }
                4 => {// Entry Type 4: Local APIC Non-maskable interrupts
                    fields.push(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const LocalAPICNonMaskableInterrupts)
                    });
                    5
                }
                5 => {// Entry Type 5: Local APIC Address Override
                    fields.push(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const LocalAPICAddressOverride)
                    });
                    12
                }
                9 => {// Entry Type 9: Processor Local x2APIC
                    fields
                        .push(unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalx2Apic) });
                    16
                }

                _ => {
                    panic!(
                        "Unrecognised record entry type: {} | length: {record_length}",
                        record_type
                    )
                }
            }
        }
        Some(Self {
            inner,
            fields,
            cores,
        })
    }
}

pub trait APICRecord: core::fmt::Debug + Sync {}

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
