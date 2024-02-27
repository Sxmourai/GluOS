use crate::{boot::hlt_loop, interrupts::msr, println};

extern "C" fn test_user() {
    panic!("Testing user");
    // unsafe{core::arch::asm!("cli")}
}

pub fn go_ring3() {
    crate::gdt::GDT.0;
    // GDT setup, Barebone TSS and stack start is already done
    //TODO Set up IDT for syscalls
    //TODO IRQ handling
    //TODO Plans for multitasking with task switching
    unsafe { jump_usermode_iret() }
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
