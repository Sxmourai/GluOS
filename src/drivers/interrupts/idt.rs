use core::cell::Cell;

use spin::RwLock;
use x86_64::structures::idt::InterruptDescriptorTable;

use crate::drivers::gdt::DOUBLE_FAULT_IST_INDEX;

use super::exceptions::*;

pub static mut IDT: Option<RwLock<InterruptDescriptorTable>> = None;

pub fn create_idt() -> InterruptDescriptorTable {
    let mut idt = InterruptDescriptorTable::new();
    idt.alignment_check.set_handler_fn(alignment_check);
    idt.bound_range_exceeded
        .set_handler_fn(bound_range_exceeded);
    idt.breakpoint.set_handler_fn(breakpoint_handler);
    idt.device_not_available
        .set_handler_fn(device_not_available);
    idt.divide_error.set_handler_fn(divide_error);
    unsafe {
        // Double fault occurs when an exception occurs while an exception function is being called...
        // If double fault fails, a triple fault is invoked which, on most hardware, cause a system reboot
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX)
    };
    idt.general_protection_fault
        .set_handler_fn(general_protection_fault);
    idt.invalid_opcode.set_handler_fn(invalid_opcode);
    idt.invalid_tss.set_handler_fn(invalid_tss);
    idt.machine_check.set_handler_fn(machine_check); // Never returns
    idt.non_maskable_interrupt
        .set_handler_fn(non_maskable_interrupt);
    idt.overflow.set_handler_fn(overflow);
    idt.page_fault.set_handler_fn(page_fault_handler);
    idt.security_exception.set_handler_fn(security_exception);
    idt.segment_not_present.set_handler_fn(segment_not_present);
    idt.simd_floating_point.set_handler_fn(simd_floating_point);
    idt.stack_segment_fault.set_handler_fn(stack_segment_fault);
    idt.virtualization.set_handler_fn(virtualization);
    idt.vmm_communication_exception
        .set_handler_fn(vmm_communication_exception);
    idt.x87_floating_point.set_handler_fn(x87_floating_point);

    // Mapping misc interrupts
    idt.debug.set_handler_fn(debug_handler);

    // HARDWARE INTERRUPTS
    crate::interrupts::hardware::setup_hardware_interrupts(&mut idt);
    return idt
}
