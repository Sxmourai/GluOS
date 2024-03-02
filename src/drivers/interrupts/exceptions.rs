use x86_64::structures::{
    idt::{InterruptStackFrame, PageFaultErrorCode},
    paging::{FrameAllocator, Page, PageTableFlags},
};

use crate::{mem_handler, memory::handler::map, println, time::sdelay};
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
    // Wait 1 second if debug mode, so that it doesn't spam asf
    #[cfg(debug_assertions)]
    for i in 0..1_000_000 {core::hint::spin_loop()}
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
    crate::log::print_trace(2);
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
/// For more infos on this exception: https://stackoverflow.com/questions/70730829/access-to-virtualization-exception-area-inside-a-guest-os
/// And maybe: https://kib.kiev.ua/kib/ia32-exceptions.pdf
pub extern "x86-interrupt" fn virtualization(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: virtualization\n{:#?}", stack_frame);
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
    error!(
        "EXCEPTION: PAGE FAULT
        Accessed Address: {:?}
        Error Code: {:?}
        Stack frame: {:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
    let page = Page::containing_address(Cr2::read());
    if error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
        // Frame is already mapped, we have to change the flags
        let flags = if error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH) {
            // Was in NO_EXECUTE
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        } else if error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE) {
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE
        } else {
            log::error!(
                "We should change the flags !, consider making an issue to support this page fault"
            );
            PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE
        };
        log::debug!("Changing flags to {:?}", flags);
        unsafe {
            x86_64::structures::paging::Mapper::update_flags(
                &mut mem_handler!().mapper,
                page,
                flags,
            )
        }
        .unwrap();
    } else {
        // Don't know what to do, so try to allocate a frame...
        log::debug!("Allocating a frame !");
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        map(page, flags);
    }
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
