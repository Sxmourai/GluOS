use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

use crate::{hlt_loop, println};

pub extern "x86-interrupt" fn alignment_check(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: alignment_check\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn divide_error(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: DIVIDE ERROR (u bad sry)\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn device_not_available(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: device_not_available\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn machine_check(stack_frame: InterruptStackFrame) -> ! {
    println!("EXCEPTION: Machine Check\n{:#?}", stack_frame);
    hlt_loop()
}
pub extern "x86-interrupt" fn non_maskable_interrupt(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: non_maskable_interrupt\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn bound_range_exceeded(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: bound_range_exceeded\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn invalid_opcode(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: invalid_opcode\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn overflow(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: overflow\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn simd_floating_point(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: simd_floating_point\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn x87_floating_point(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: x87_floating_point\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn general_protection_fault(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: general_protection_fault\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn invalid_tss(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: invalid_tss\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn security_exception(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: security_exception\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn segment_not_present(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: segment_not_present\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn stack_segment_fault(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: stack_segment_fault\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn virtualization(stack_frame: InterruptStackFrame) {
    //TODO: Do some research on wtf when this is called
    panic!("EXCEPTION: virtualization\n{:#?}", stack_frame); // Idk what this means
}
pub extern "x86-interrupt" fn vmm_communication_exception(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: vmm_communication_exception\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}

pub extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    println!("DEBUG INTERRUPT\n{:#?}", stack_frame);
}

pub extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    println!("Accessed Address: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}
