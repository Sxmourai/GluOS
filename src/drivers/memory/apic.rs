//! https://wiki.osdev.org/APIC

use x86_64::registers::model_specific::Msr;
static IA32_APIC_BASE_MSR:Msr        = Msr::new(0x1B);
static IA32_APIC_BASE_MSR_BSP:Msr    = Msr::new(0x100); // Processor is a BSP
static IA32_APIC_BASE_MSR_ENABLE:Msr = Msr::new(0x800);

// /** returns a 'true' value if the CPU supports APIC
//  *  and if the local APIC hasn't been disabled in MSRs
//  *  note that this requires CPUID to be supported.
//  */
// fn check_apic() -> bool {
//     let edx = cpuid!(1);
//     return edx & CPUID_FEAT_EDX_APIC;
//  }

/* Set the physical address for local APIC registers */
// fn cpu_set_apic_base(apic:&RMADT) {
//     let edx:u32 = 0;
//     let eax:u32 = (apic & 0xfffff0000) | IA32_APIC_BASE_MSR_ENABLE;
//   //TODO Get __PHYSICAL_MEMORY_EXTENSION__
// //  #ifdef __PHYSICAL_MEMORY_EXTENSION__
// //     edx = (apic >> 32) & 0x0f;
// //  #endif
//     unsafe { IA32_APIC_BASE_MSR.write(eax) }
//  }












// 