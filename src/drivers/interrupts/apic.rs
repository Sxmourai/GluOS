use x86_64::PhysAddr;

use crate::descriptor_tables;

use super::msr::{set_msr, get_msr};

const IA32_APIC_BASE_MSR: u8 = 0x1B;
const IA32_APIC_BASE_MSR_BSP: u16 = 0x100;
const IA32_APIC_BASE_MSR_ENABLE: u16 = 0x800;

/// Tries to init APIC
pub unsafe fn init() -> Result<ApicHandler, ApicInitError>{
    if check_apic() {
        /* Section 11.4.1 of 3rd volume of Intel SDM recommends mapping the base address page as strong uncacheable for correct APIC operation. */
        /* Hardware enable the Local APIC if it wasn't enabled */
        unsafe {cpu_set_apic_base(&cpu_get_apic_base())};
        /* Set the Spurious Interrupt Vector Register bit 8 to start receiving interrupts */
        let regs_addr = unsafe{descriptor_tables!().madt.inner.local_apic_addr};
        let spurious = regs_addr + 0xF0;
        
        //TODO Map these to virtual address
        let reg = unsafe { &mut *(spurious as *mut u32) };
        *reg = *reg | 0x100; // write_reg(0xF0, read_reg(0xF0) | 0x100);
        Ok(ApicHandler {})
    } else {
        Err(ApicInitError::NotSupported)
    }
}

fn check_apic() -> bool {
    let id = raw_cpuid::CpuId::new();
    id.get_feature_info().unwrap().has_apic()
}


/* Set the physical address for local APIC registers */
unsafe fn cpu_set_apic_base(apic: &PhysAddr) {
    let base = (apic.as_u64() & 0xfffff0000) | IA32_APIC_BASE_MSR_ENABLE as u64;
    unsafe {set_msr(IA32_APIC_BASE_MSR as u32, base)}
}
  
 /**
  * Get the physical address of the APIC registers page
  * make sure you map it to virtual memory ;)
  */
//TODO Handle __PHYSICAL__MEMORY_EXTENSION__ (https://wiki.osdev.org/APIC)
unsafe fn cpu_get_apic_base() -> PhysAddr {
    let ptr = unsafe{get_msr(IA32_APIC_BASE_MSR as u32)} & 0xfffff000;
    PhysAddr::new(ptr)
}
#[derive(Debug)]
pub enum ApicInitError {
    NotSupported
}
pub struct ApicHandler {

}
#[repr(C, packed)]
pub struct RawAPIC {

}