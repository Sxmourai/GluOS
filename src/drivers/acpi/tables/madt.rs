use alloc::vec::Vec;
use log::debug;

use crate::{dbg, serial_println};

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
        return f.debug_struct("RawMADT")
            .field("h", &self.h)
            .field("local_apic_addr", &local_apic_addr)
            .field("flags", &flags)
            .finish()
    }
}
#[derive(Debug)]
pub struct MADT {
    pub inner: &'static RawMADT,
    pub fields: Vec<ApicRecord>,
    pub cores: Vec<Core>, // (core id, apic id)
}
impl MADT {
    pub fn new(bytes: &[u8]) -> Option<Self> {
        let inner = unsafe { &*(bytes.as_ptr() as *const RawMADT) };
        let mut fields = Vec::new();
        let mut cores = Vec::new();
        let mut start_idx = core::mem::size_of::<RawMADT>(); // Fields start at end of RawMADT
        while start_idx + 1 < bytes.len() {
            let record_type = &bytes[start_idx];
            let record_length = &bytes[start_idx + 1];
            start_idx += 2;
            let record = match record_type {
                0 => {
                    // Entry Type 0: Processor Local APIC
                    let proc_local_apic =
                        unsafe { &*(bytes[start_idx..].as_ptr() as *const ProcLocalAPIC) };
                    cores.push((cores.len(), proc_local_apic.acpi_proc_id));
                    ApicRecord::ProcLocalAPIC(proc_local_apic)
                }
                1 => {
                    // Entry Type 1: I/O APIC
                    ApicRecord::IOAPIC(unsafe { &*(bytes[start_idx..].as_ptr() as *const IOAPIC) })
                }
                2 => {
                    // Entry Type 2: IO/APIC Interrupt Source Override
                    ApicRecord::IOAPICInterruptSourceOverride(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const IOAPICInterruptSourceOverride)
                    })
                }
                3 => {
                    // Entry type 3: IO/APIC Non-maskable interrupt source
                    ApicRecord::IOAPICNonMaskableInterruptSource(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const IOAPICNonMaskableInterruptSource)
                    })
                }
                4 => {
                    // Entry Type 4: Local APIC Non-maskable interrupts
                    ApicRecord::LocalAPICNonMaskableInterrupts(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const LocalAPICNonMaskableInterrupts)
                    })
                }
                5 => {
                    // Entry Type 5: Local APIC Address Override
                    ApicRecord::LocalAPICAddressOverride(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const LocalAPICAddressOverride)
                    })
                }
                9 => {
                    // Entry Type 9: Processor Local x2APIC
                    ApicRecord::ProcLocalx2Apic(unsafe {
                        &*(bytes[start_idx..].as_ptr() as *const ProcLocalx2Apic)
                    })
                }

                _ => {
                    log::error!(
                        "Unrecognised record entry type: {record_type} | length: {record_length}",
                    );
                    continue;
                }
            };
            start_idx += *record_length as usize - 2;
            fields.push(record);
        }
        return Some(Self {
            inner,
            fields,
            cores,
        })
    }
}

#[derive(Debug, Clone)]
pub enum ApicRecord {
    ProcLocalAPIC(&'static ProcLocalAPIC),
    IOAPIC(&'static IOAPIC),
    IOAPICInterruptSourceOverride(&'static IOAPICInterruptSourceOverride),
    IOAPICNonMaskableInterruptSource(&'static IOAPICNonMaskableInterruptSource),
    LocalAPICNonMaskableInterrupts(&'static LocalAPICNonMaskableInterrupts),
    LocalAPICAddressOverride(&'static LocalAPICAddressOverride),
    ProcLocalx2Apic(&'static ProcLocalx2Apic),
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct ProcLocalAPIC {
    // Entry Type 0: Processor Local APIC https://wiki.osdev.org/MADT#Entry_Type_0:_Processor_Local_APIC
    pub acpi_proc_id: u8,
    pub apic_id: u8,
    pub flags: u32, // (bit 0 = Processor Enabled) (bit 1 = Online Capable)
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct IOAPIC {
    // Entry Type 1: I/O APIC
    pub io_apic_id: u8,
    pub reserved: u8,
    pub io_apic_address: u32,
    pub global_system_interrupt_base: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
// This explains how IRQ sources are mapped to global system interrupts
pub struct IOAPICInterruptSourceOverride {
    // Entry Type 2: IO/APIC Interrupt Source Override
    bus_source: u8,
    irq_source: u8,
    global_system_interrupt: u32,
    flags: u16,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct IOAPICNonMaskableInterruptSource {
    // Entry type 3: IO/APIC Non-maskable interrupt source
    nmi_source: u8,
    reserved: u8,
    flags: u16,
    global_system_interrupt: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct LocalAPICNonMaskableInterrupts {
    // Entry Type 4: Local APIC Non-maskable interrupts
    acpi_proc_id: u8, // (0xFF means all processors)
    flags: u16,
    lint: u8,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct LocalAPICAddressOverride {
    // Entry Type 5: Local APIC Address Override
    reserved: u16,
    phys_addr_local_apic: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct ProcLocalx2Apic {
    // Entry Type 9: Processor Local x2APIC
    reserved: u16,
    proc_local_x2apic_id: u32,
    flags: u32,
    acpi_id: u32,
}
