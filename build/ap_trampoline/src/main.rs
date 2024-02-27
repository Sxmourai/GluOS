#![no_std]
#![no_main]

#[no_mangle]
fn _start() {
    unsafe {
        core::ptr::write_volatile(0xb8002 as *mut u8, b'g');
        core::ptr::write_volatile(0xb8003 as *mut u8, b'g');
        core::arch::asm!("int3");
    }
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe { *(0xb8002 as *mut u8) = 'a' as u8 };
    loop {}
}
