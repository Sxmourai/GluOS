use bit_field::BitField;
use raw_cpuid::CpuId;
use x86_64::{
    structures::paging::{Mapper, PageTableFlags, Size4KiB, Translate},
    PhysAddr, VirtAddr,
};

use crate::{
    descriptor_tables,
    interrupts::apic::Offset,
    mem_handler,
    time::{mdelay, udelay},
};

pub static AP_TRAMPOLINE: &[u8] = include_bytes!("../../../build/ap_trampoline/ap_trampoline.bin");

/// Returns the amount of launched CPUs
/// TODO Return a Vec of Cpu structs that could have an id or smth
#[must_use]
pub fn init_other_units() -> u8 {
    //TODO AP.len() or 1 ?
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    // mem_handler!().mapper.tr
    crate::dbg!();
    unsafe {
        mem_handler!().map_frame(
            x86_64::structures::paging::Page::<Size4KiB>::containing_address(VirtAddr::new(0x8000)),
            x86_64::structures::paging::PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(
                0x8000,
            )),
            flags,
        )
    };
    crate::dbg!();
    unsafe {
        core::ptr::copy_nonoverlapping(
            AP_TRAMPOLINE.as_ptr(),
            0x8000 as *mut u8,
            AP_TRAMPOLINE.len(),
        );
    };
    crate::dbg!();
    let trampoline =
        unsafe { core::slice::from_raw_parts(0x8000 as *const u8, AP_TRAMPOLINE.len()) };
    debug_assert_eq!(trampoline, AP_TRAMPOLINE);
    unsafe { core::arch::asm!("jmp {:r}", in(reg) 0x8000) };

    let bsp_id = CpuId::new()
        .get_feature_info()
        .unwrap()
        .initial_local_apic_id() as _; //TODO unwrap -> return 0
    crate::dbg!(bsp_id);
    let num_cores = descriptor_tables!().num_core();
    log::debug!("Initializing {} cores", num_cores);
    for i in 0..num_cores {
        assert!(i < u8::MAX as _);
        // do not start BSP that's already running this code
        if i == bsp_id {
            continue;
        }
        let mut apic = crate::interrupts::apic::get();
        // clear APIC errors
        apic.write(Offset::ErrorStatus, 0);
        // select AP
        apic.write(
            Offset::InterruptCommandHigh,
            (apic.read(Offset::InterruptCommandHigh) & 0x00ff_ffff) | (i as u32) << 24,
        );
        // trigger INIT IPI
        apic.write(
            Offset::InterruptCommandLow,
            (apic.read(Offset::InterruptCommandLow) & 0xfff0_0000) | 0x0000_C500,
        );
        wait_for_delivery(&mut apic);
        // select AP
        apic.write(
            Offset::InterruptCommandHigh,
            (apic.read(Offset::InterruptCommandHigh) & 0x00ff_ffff) | (i as u32) << 24,
        );
        // Deassert
        apic.write(
            Offset::InterruptCommandLow,
            (apic.read(Offset::InterruptCommandLow) & 0xfff0_0000) | 0x0000_8500,
        );
        wait_for_delivery(&mut apic);
        mdelay(10);
        debug_assert!(apic.read(Offset::ErrorStatus) == 0);
        // send STARTUP IPI (twice)
        for j in 0..2 {
            apic.write(Offset::ErrorStatus, 0);
            // select AP
            apic.write(
                Offset::InterruptCommandHigh,
                (apic.read(Offset::InterruptCommandHigh) & 0x00ff_ffff) | (i as u32) << 24,
            );
            // trigger STARTUP IPI for 0800:0000
            apic.write(
                Offset::InterruptCommandLow,
                (apic.read(Offset::InterruptCommandLow) & 0xfff0_f800) | 0x0000_0608,
            );
            udelay(200);
            wait_for_delivery(&mut apic);
        }
        debug_assert!(apic.read(Offset::ErrorStatus) == 0);
    }
    unsafe { core::ptr::read((0x8000 - 2) as _) }
}
fn wait_for_delivery(apic: &mut super::apic::Apic) {
    loop {
        if !apic.read(Offset::InterruptCommandLow).get_bit(12) {
            break;
        }
        core::hint::spin_loop();
    }
}
