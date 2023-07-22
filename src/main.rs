#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_runner"]

extern crate kernel;
extern crate bootloader;
extern crate x86_64;
extern crate alloc;

use core::panic::PanicInfo;
use crate::kernel::{serial_println, hlt_loop};
use alloc::vec::Vec;
use bootloader::{BootInfo, entry_point};
use kernel::{serial_print, println, state::get_mem_handler, terminal::console::{pretty_print, CONSOLE}, writer::{outb, inb}};
use pci_ids::SubSystem;
use x86_64::{VirtAddr, instructions::hlt};

// Function to move the cursor to a specific position in the VGA buffer
pub fn move_cursor_to(x: u16, y: u16) {
    let pos: u16 = y * 80 + x;
    // serial_println!("{:b} - {}", y, y);
    serial_println!("{} shifted> {} then> {} | {}", pos, (pos >> 8) & 0xFF, (pos & 0xFF), y);
    unsafe {
        // Select VGA controller register: Index 14
        // This is the high byte of the cursor's position
        outb(0x3D4, 0x0F);
        outb(0x3D5, ((pos & 0xFF)).try_into().unwrap());
        outb(0x3D4, 0x0E);
        outb(0x3D5, (((pos >> 8) & 0xFF)).try_into().unwrap());
    }

    // unsafe {
    //     // Select VGA controller register: Index 14
    //     // This is the high byte of the cursor's position
    //     for i in 0..u16::MAX {
    //         outb(0x3D4, i);
    //         inb(0x3D5);
    //     }
    //     outb(0x3D4, 14);
    //     // Send the high byte of the cursor's position
    //     outb(0x3D5, ((pos >> 8) & 0xFF));

    //     // Select VGA controller register: Index 15
    //     // This is the low byte of the cursor's position
    //     // outb(0x3D4, 15);
    //     // Send the low byte of the cursor's position
    //     // outb(0x3D5, (pos & 0xFF));
    // }
}

// Sets the entry point of our kernel for the bootloader. This means we can have the 'boot_info' variable which stores some crucial info
entry_point!(kernel_main);
// Main function of our kernel (1 func to start when boot if not in test mode). Never returns, because kernel runs until machine poweroff
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // Initialize & boot the device and kernel
    serial_print!("START");
    kernel::interrupts::init();
    loop {
        let mut i = 0;
        loop {
            move_cursor_to(0, i);
            if i > 25 {
                break
            }
            hlt();hlt();hlt();
            
            i += 1;
            // serial_print!("{} - ", i);
        }
        serial_println!("REV");
        for j in (0..25).rev() {
            move_cursor_to(0, j);
            // serial_print!("{} - ", j);
            hlt();hlt();hlt();
        }
    }
    kernel::boot(boot_info);
    println!("HI");
    // unsafe {
        //     outb(0x3D4, 0x0A); // Select the cursor start register
    //     let mut cursor_start = inb(0x3D5); // Read the current value

    //     // Disable blinking: Set bit 5 (cursor off) and clear bit 4 (no blinking)
    //     cursor_start &= !(1 << 4); // Clear bit 4
    //     cursor_start |= 1 << 5; // Set bit 5
    //     serial_println!("{:b} < {:b}", cursor_start, u8::MAX);
    //     outb(0x3D5, cursor_start as u8); // Write the updated value back
    // }
    // for y in 0..25 {
    //     for x in 0..80 {
    //         move_cursor_to(x, y);
    //         // Add a delay to observe the cursor movement (not recommended in real code)
    //         for _ in 0..100000 {}
    //     }
    // }

    // for device in kernel::pci::pci_device_iter() {
    //     if device.class == 1 {
    //         serial_println!("{:?}", device.subclass);
    //     }
    // }

    // let to_print = unsafe { rsdp::Rsdp::search_for_on_bios(&mut kernel::state::get_mem_handler().get_mut().frame_allocator) }.unwrap();
    // crate::serial_println!("Phys start: {:?}", to_print.physical_start());

    // x86_64::PhysAddr::new(0x40E)
    // unsafe { & *(x86_64::PhysAddr::new(0x40E).as_u64() as *const u16) }
    // unsafe {core::ptr::read_volatile(0x40E as *const u16)}
    // crate::serial_println!("{} - {}",0x000E0000 as usize, 0x000FFFFF);
    // crate::serial_println!("{:?}", unsafe{& *(0x000FFFFF as *const u16)});
    
    // kernel::boot::end()
    // Enter a 'sleep' phase (a.k.a. finished booting)
    hlt_loop()
}


#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("KERNEL PANIC:{}", info);
    kernel::hlt_loop();
}
