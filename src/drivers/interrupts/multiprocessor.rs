use x86_64::{
    structures::paging::{Mapper, PageTableFlags, Size4KiB},
    VirtAddr,
};

use crate::mem_handler;

pub static AP_TRAMPOLINE: &[u8] = include_bytes!("../../../build/ap_trampoline/ap_trampoline.bin");

/// Returns the amount of launched CPUs
/// TODO Return a Vec of Cpu structs that could have an id or smth
pub fn init_other_units() -> usize {
    return 0;
    let addr = core::ptr::addr_of!(AP_TRAMPOLINE);
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    unsafe {
        mem_handler!().mapper.update_flags(
            x86_64::structures::paging::Page::<Size4KiB>::containing_address(VirtAddr::new(
                addr as u64,
            )),
            flags,
        )
    };
    // return 0;
    //TODO handle gracefully program shut down
    // We have to assert address < 4G
    // Write ff e0 = jmp rax, to return from program
    // mov rax, 0x500 = b8 00 05 00 00
    // b8 = mov
    // To get hex code of asm instructions: echo mov rax, 0x586370 > a.asm && nasm -felf64 a.asm && objdump a.o -d
    // Btw this is not at all how you start up other cpu's, this was for testing my ap_trampoline... And it works =)
    let program_start = core::ptr::addr_of!(AP_TRAMPOLINE);
    crate::dbg!(program_start);
    unsafe {
        core::arch::asm!("jmp {:r}", in(reg) program_start);
    };

    todo!()
}
