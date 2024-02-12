#![no_std]
#![no_main]

#[no_mangle]
fn _start() {
    unsafe {*(0xb8000 as *mut u8) = 'g' as u8};
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    unsafe {*(0xb8000 as *mut u8) = 'a' as u8};
    loop {}
}