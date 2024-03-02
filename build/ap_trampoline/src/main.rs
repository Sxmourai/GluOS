#![no_std]
#![no_main]

core::arch::global_asm! ("
.code16
    cli
    cld
    ljmp 0, 0x8010
    .align 16
_L8010_rest:
    mov al, 'g'
    mov [0xb8000], al
    mov [0xb8001], al
    // TODO: Jump to Rust
spin:
    jmp spin
// So that the compiler is happy, but unreachable
.code64
    ",
);

#[no_mangle]
fn _start() {
    unsafe{core::hint::unreachable_unchecked()};
    // unsafe {
    //     core::ptr::write_volatile(0xb8002 as *mut u8, b'g');
    //     core::ptr::write_volatile(0xb8003 as *mut u8, b'g');
    //     core::arch::asm!("int3");
    // }
    // loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe{core::hint::unreachable_unchecked()};
    // unsafe { *(0xb8002 as *mut u8) = 'a' as u8 };
    loop {}
}
