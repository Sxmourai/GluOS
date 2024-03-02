use bit_field::BitField;
use x86_64::{
    registers::segmentation::{Segment, CS, DS},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable},
        paging::{Page, PageTableFlags, PhysFrame},
    },
    PhysAddr, VirtAddr,
};

use crate::{
    boot::hlt_loop, gdt::{get_gdt, Selectors, GDT}, interrupts::msr, mem_handler, memory::handler::{map, map_frame}, println
};

extern "C" fn test_user() {
    panic!("Testing user");
    // unsafe{core::arch::asm!("cli")}
}
/// From https://github.com/nikofil/rust-os/blob/master/kernel/src/scheduler.rs
pub fn go_ring3() {
    return;
    // TODO setup_separate_page_table();
    let (gdt, s) = get_gdt();
    let (cs, ds) = (s.user_code_segment, s.user_data_segment);
    assert!(cs.0 & x86_64::PrivilegeLevel::Ring3 as u16!=0);
    assert!(ds.0 & x86_64::PrivilegeLevel::Ring3 as u16!=0);
    crate::dbg!();
    unsafe { x86_64::instructions::segmentation::DS::set_reg(ds) };
    crate::dbg!();
    x86_64::instructions::tlb::flush_all(); // flush the TLB after address-space switch
    //TODO Set up IDT for syscalls
    //TODO Plans for multitasking with task switching
    crate::dbg!();
    let addr = userspace_prog_1 as *const () as u64;
    crate::dbg!();
    let page = Page::containing_address(VirtAddr::new(addr));
    let frame = PhysFrame::containing_address(PhysAddr::new(addr));
    let flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    map_frame(page, frame, flags);
    crate::dbg!();
    unsafe { core::arch::asm!("
    push rax   // stack segment
    push rsi   // rsp
    push 0x200 // rflags (only interrupt bit set)
    push rdx   // code segment
    push rdi   // ret to virtual addr
    iretq",
    in("rdi") addr, in("rsi") 0x0080_0000, in("dx") cs.0, in("ax") ds.0
    );}
}
#[inline(always)]
#[must_use] pub fn get_usermode_segs(gdt_segs: &Selectors) -> (u16, u16) {
    // set ds and tss, return cs and ds
    let (mut cs, mut ds) = (gdt_segs.user_code_segment, gdt_segs.user_data_segment);
    cs.0 |= x86_64::PrivilegeLevel::Ring3 as u16;
    ds.0 |= x86_64::PrivilegeLevel::Ring3 as u16;
    (cs.0, ds.0)
}
/// From https://nfil.dev/kernel/rust/coding/rust-kernel-to-userspace-and-back/
/// Should setup separate stack and paging
fn setup_separate_page_table() {
    let mut pt = x86_64::structures::paging::PageTable::new(); // allocate the master PT struct
    let mut pts = pt.iter_mut();
    let mut entry0 = unsafe { pts.next().unwrap_unchecked() };
    let user_flags =
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
    entry0.set_frame(
        unsafe { mem_handler!().alloc(user_flags) }.unwrap(),
        user_flags,
    );
    let mut pt0 = unsafe { pts.next().unwrap_unchecked() }; // get the child PT we just allocated
                                                            // copy over the entries 3, 4, 5, 6 from the equivalent child PT that is currently in use
    let mut cur_pt0 = mem_handler!().mapper.level_4_table().iter().skip(3);
    let mut pts = pts.skip(1); // Skip 1 cuz we already done 2 next's above
    for page_idx in 3..=6 {
        *pts.next().unwrap() = cur_pt0.next().unwrap().clone();
    }
    let program_addr = PhysAddr::new(userspace_prog_1 as *const () as u64);
    let page_phys_start = *program_addr.as_u64().set_bits(0..12, 0); // zero out page offset to get which page we should map
    let fn_page_offset = program_addr.as_u64() - page_phys_start; // offset of function from page start
    let userspace_fn_virt_base = 0x0040_0000; // target virtual address of page
    let userspace_fn_virt = userspace_fn_virt_base + fn_page_offset; // target virtual address of function
    todo!()
}
pub unsafe fn userspace_prog_1() {
    unsafe {
        core::arch::asm!(
            "
        nop
        nop
        nop
        int3
    "
        );
    }
}

unsafe fn jump_usermode_iret() {
    unsafe {
        core::arch::asm!("
        mov ax, (4 * 8) | 3
        mov ds, ax
        mov es, ax
        mov fs, ax
        mov gs, ax
        mov eax, esp
        push (4 * 8) | 3
        push word ptr[eax]
        pushf
        push (3 * 8) | 3
        push {test_user_function}
        iretq
        ", test_user_function = sym test_user,
        );
    }
}

// unsafe fn jump_usermode_sysretq() {
//     unsafe{msr::set_msr(0xc0000081, (1<<3)|((3<<3)|3))};
//     unsafe{msr::set_msr(0xc0000082, 0x00180008)};

//     unsafe{core::arch::asm!("
// 	mov ecx, {test_user_function} //to be loaded into RIP
// 	mov r11, 0x202 //to be loaded into EFLAGS
//     hlt
// 	sysretq
//     ",
//     test_user_function = sym test_user,
//     )}
// }

// extern "C" fn print_foo() {
//     println!("FOO")
// }

// unsafe fn jump_usermode_sysexit() {
//     print_foo();
//     unsafe{
//         core::arch::asm!("call {f}", f=sym print_foo);
//     }
//     let function_ptr: *const () = unsafe{core::mem::transmute(print_foo as *const ())};
//     unsafe{core::arch::asm!("
// 	mov ax, (4 * 8) | 3 //user data segment with RPL 3
// 	mov ds, ax
// 	mov es, ax
// 	mov fs, ax
// 	mov gs, ax //sysexit sets SS

// 	//setup wrmsr inputs
// 	xor edx, edx //not necessary//set to 0
// 	mov eax, 0x8 //the segments are computed as follows: CS=MSR+0x10 (0x8+0x10=0x18), SS=MSR+0x18 (0x8+0x18=0x20).
// 	mov ecx, 0x174 //MSR specifier: IA32_SYSENTER_CS
// 	wrmsr //set sysexit segments

// 	//setup sysexit inputs
// 	mov rdx, {test_user_function} //to be loaded into EIP
// 	mov ecx, esp //to be loaded into ESP
// 	sysexit
//     ",
//     test_user_function = in(reg) function_ptr,
//     )}
// }
