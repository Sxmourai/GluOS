use raw_cpuid::CpuId;
use x86_64::PhysAddr;

use crate::{dbg, descriptor_tables, memory::tables::madt::ApicRecord};

use super::msr::{get_msr, set_msr};

const IA32_APIC_BASE_MSR: u8 = 0x1B;
const IA32_APIC_BASE_MSR_BSP: u16 = 0x100;
const IA32_APIC_BASE_MSR_ENABLE: u16 = 0x800;

/// Tries to init APIC
pub unsafe fn init() -> Result<(), ApicInitError> {
    if check_apic() {
        let lapic_addr = unsafe { descriptor_tables!().madt.inner.local_apic_addr };
        init_local_apic(lapic_addr);
        init_io_apic();
        bsp_init(lapic_addr);
        Ok(())
    } else {
        Err(ApicInitError::NotSupported)
    }
}

fn init_local_apic(lapic_addr: u32) {
    /* Section 11.4.1 of 3rd volume of Intel SDM recommends mapping the base address page as strong uncacheable for correct APIC operation. */
    /* Hardware enable the Local APIC if it wasn't enabled */
    unsafe { cpu_set_apic_base(&cpu_get_apic_base()) };
    /* Set the Spurious Interrupt Vector Register bit 8 to start receiving interrupts */

    //TODO Map these to virtual address
    let regs = unsafe { &mut *(lapic_addr as *mut LApicRegs) };
    regs.supious_interrupt_vector.inner = regs.supious_interrupt_vector.inner | 0x100;
}

fn init_io_apic() {
    let mut io_apic = None;
    for record in unsafe { &descriptor_tables!().madt.fields } {
        match record {
            ApicRecord::IOAPIC(ioapic) => {
                io_apic = Some(ioapic); // !Can have multiple io_apic, this iznogood
            }
            ApicRecord::ProcLocalAPIC(proclocalapic) => {}
            _ => {}
        }
    }
    // Should have ioapic
    let io_apic = io_apic.unwrap();
    //TODO Initialize io apic
}

extern "C" fn ap_startup(apic_id: isize) -> ! {
    loop {}
    
}

fn bsp_init(lapic_addr: u32) {
    let mut ap_running = 0;
    let mut bsp_id = 0;

    let mut bsp_done = 0;
    // get the BSP's Local APIC ID
    // unsafe{core::arch::asm!(
    //     "mov {bspid:e}, eax; cpuid; shl $24, ebx;", 
    //     bspid = inout(reg) bsp_id,
    // )};
    let numcores = unsafe {descriptor_tables!().num_core()} as u32;
    let lapic_ids = unsafe {&descriptor_tables!().madt.cores};
    let l = lapic_addr; // Makes an alias to call functions and take less place
    // copy the AP trampoline code to a fixed address in low conventional memory (to address 0x0800:0x0000)

    for i in 0..numcores {
        // do not start BSP, that's already running this code
        if lapic_ids[i as usize].0 == bsp_id {continue}
        // --------------send INIT IPI------------
        // clear APIC errors
        unsafe{write_lapic(l, 0x280, 0)}
        select_ap(l, i);
        // trigger INIT IPI
        unsafe{write_lapic(l,0x300, (read_lapic(l,0x300) & 0xfff00000) | 0x00C500)}
        wait_for_delivery(l);        
        select_ap(l, i);
        // deassert
        unsafe{write_lapic(l, 0x300, (read_lapic(l, 0x300) & 0xfff00000) | 0x008500)}
        wait_for_delivery(l);
        // mdelay(10)
    	// send STARTUP IPI (twice)
        for j in 0..2 {
            // clear APIC errors
            unsafe { write_lapic(l, 0x280, 0) };
            select_ap(l, i);
            // trigger STARTUP IPI for 0800:0000
            unsafe{write_lapic(l,0x300, (read_lapic(l, 0x300) & 0xfff0f800) | 0x000608)}
            //udelay(200);
            wait_for_delivery(l);
        }
    }
    // release the AP spinlocks
    bsp_done = 1;
    dbg!(ap_running);
}
// select AP
fn select_ap(lapic_addr: u32, core:u32) {
    unsafe{write_lapic(lapic_addr,0x310, (read_lapic(lapic_addr, 0x310) & 0x00ffffff) | ((core)<<24))}
}
fn wait_for_delivery(lapic_addr: u32) {
    unsafe {
        core::arch::asm!("pause", "");
        while read_lapic(lapic_addr, 0x300) & (1<<12)!=0 {
            core::arch::asm!("pause", "");
        }
    }
}
unsafe fn write_lapic(lapic_addr: u32, reg: u32, val: u32) {
    unsafe{core::ptr::write_volatile(&mut *((lapic_addr+reg) as *mut u32), val)}
}
unsafe fn read_lapic(lapic_addr: u32, reg: u32) -> u32 {
    unsafe{core::ptr::read_volatile(&mut *((lapic_addr+reg) as *mut u32))}
}
fn check_apic() -> bool {
    let id = raw_cpuid::CpuId::new();
    id.get_feature_info().unwrap().has_apic()
}

fn write_ioapic_reg(apic_base: u64, offset: u8, val: u32) {
    let mut ioregsel = unsafe { &mut *(apic_base as *mut u8) };
    unsafe { core::ptr::write_volatile(ioregsel, offset) };
    let mut iowin = unsafe { &mut *((10 + apic_base) as *mut u32) };
    unsafe { core::ptr::write_volatile(iowin, val) };
}

/* Set the physical address for local APIC registers */
unsafe fn cpu_set_apic_base(apic: &PhysAddr) {
    let base = (apic.as_u64() & 0xfffff0000) | IA32_APIC_BASE_MSR_ENABLE as u64;
    unsafe { set_msr(IA32_APIC_BASE_MSR as u32, base) }
}

/**
 * Get the physical address of the APIC registers page
 * make sure you map it to virtual memory ;)
 */
//TODO Handle __PHYSICAL__MEMORY_EXTENSION__ (https://wiki.osdev.org/APIC)
unsafe fn cpu_get_apic_base() -> PhysAddr {
    let ptr = unsafe { get_msr(IA32_APIC_BASE_MSR as u32) } & 0xfffff000;
    PhysAddr::new(ptr)
}
#[derive(Debug)]
pub enum ApicInitError {
    NotSupported,
}
#[repr(C, packed)]
pub struct ApicReg {
    inner: u32,
    align: [u8; 12], // Regs are aligned on 16 bytes
}
#[repr(C, packed)]
pub struct LApicRegs {
    reserved: [ApicReg; 2],
    pub lapic_id: ApicReg,      //Read/Write
    pub lapic_version: ApicReg, // Read
    reserved_: [ApicReg; 4],
    pub task_priority: ApicReg,            // Read/Write
    pub arbitration_priority: ApicReg,     // Read
    pub processor_priority: ApicReg,       // Read
    pub eoi: ApicReg, // Write value 0 for end of interrupt, !=0 general protect fault
    pub remote_read: ApicReg, // Read
    pub logical_destination: ApicReg, // Read/Write
    pub destination_format: ApicReg, // Read/write
    pub supious_interrupt_vector: ApicReg, // Read/write
    pub in_service: [ApicReg; 8], // Read
    pub trigger_mode: [ApicReg; 8], // Read
    pub interrupt_request: [ApicReg; 8], // Read
    pub error_status: ApicReg, // Read
    reserved_1: [ApicReg; 6],
    pub lvt_corrected_machine_check_interrupt: ApicReg, // Read/write
    pub interrupt_command_register: [ApicReg; 2],       // Read/write
    pub lvt_timer: ApicReg,                             // Read/write
    pub lvt_thermal_sensor: ApicReg,                    // Read/write
    pub lvt_performance_monitoring_counters: ApicReg,   // Read/write
    pub lvt_lint0: ApicReg,                             // Read/write// Read/write
    pub lvt_lint1: ApicReg,
    pub lvt_error: ApicReg,           // Read/write
    pub initial_count_timer: ApicReg, // Read/write
    pub current_count_timer: ApicReg, // Read
    reserved_2: [ApicReg; 3],
    pub divide_config: ApicReg, // Read/write
}

#[repr(C, packed)]
pub struct IOApicRegs {
    id: u32, // Get/set the IO APIC's id in bits 24-27. All other bits are reserved
    version_n_max_redirection: u32, // Get the version in bits 0-7. Get the maximum amount of redirection entries in bits 16-23. All other bits are reserved. Read only
    arbitration_priority: u32, // Get the arbitration priority in bits 24-27. All other bits are reserved. Read only.
    unknown: [u32; 8],
    redirection_entries: [u32; 47], // Contains a list of redirection entries. They can be read from and written to. Each entries uses two addresses, e.g. 0x12 and 0x13
}
//TODO Redirections
