use x86_64::structures::{idt::{InterruptStackFrame, PageFaultErrorCode}, paging::{FrameAllocator, PageTableFlags, Page}};

use crate::{println, mem_handler, memory::handler::map};
use log::error;

pub extern "x86-interrupt" fn alignment_check(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: alignment_check\n{:#?}\nError code: {}",
        stack_frame, error_code
    );
}
pub extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn divide_error(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DIVIDE ERROR (u bad sry)\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn device_not_available(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: device_not_available\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn machine_check(stack_frame: InterruptStackFrame) -> ! {
    panic!("EXCEPTION: Machine Check\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn non_maskable_interrupt(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: non_maskable_interrupt\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn bound_range_exceeded(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: bound_range_exceeded\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn invalid_opcode(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: invalid_opcode\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn overflow(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: overflow\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn simd_floating_point(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: simd_floating_point\n{:#?}", stack_frame);
}
pub extern "x86-interrupt" fn x87_floating_point(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: x87_floating_point\n{:#?}", stack_frame);
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
    error!(
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
    //TODO Map a page to a frame when page fault
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    unsafe { map(Page::containing_address(Cr2::read()), flags) };
    error!(
        "EXCEPTION: PAGE FAULT
    Accessed Address: {:?}
    Error Code: {:?}
    Stack frame: {:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}

// pub fn map_phys_memory(location: u64, size: usize, end_page:u64) -> &'static [u8] {
//     let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
//     let phys_frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(location));
//     let mut mem_handler = unsafe { crate::state::STATE.get_mem_handler() };
//     let mut mem_h = mem_handler.get_mut();
//     let page = Page::containing_address(VirtAddr::new(end_page));
//     unsafe { mem_h.mapper.map_to(page, phys_frame, flags, &mut mem_h.frame_allocator).unwrap().flush() };

//     let addr = location-phys_frame.start_address().as_u64() + page.start_address().as_u64();

//     // err!("Physical frame_adress: {:x}\t-\tLocation: {:x}\nComputed location {:x}\t-\tFrame to page: {:x} (Provided (unaligned): {:x})", phys_frame.start_address().as_u64(), location, addr, page.start_address().as_u64(),end_page);
//     unsafe { read_memory(addr as *const u8, size) }
// }
