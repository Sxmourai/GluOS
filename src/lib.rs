#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(test::runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(linkage)]
#![feature(naked_functions)]
#![cfg_attr(debug_assertions, allow(dead_code, unused))]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod bit_manipulation;
pub mod boot;
pub mod drivers;
pub mod state;
pub mod test;
pub mod user;


pub use drivers::*;


#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test::panic_handler(info)
}


//TODO: Remove the need for these
extern crate alloc; // Lib which stores some useful structs on the heap / smart pointers from stdlib like Vec, String, Box...
extern crate bootloader; // The bootloader crate, usefull for boot_info, paging and other stuff
extern crate hashbrown;
extern crate lazy_static; // Useful to declare some static without using only 'const fn ...'
extern crate pc_keyboard; // Transforms keyboard scancode (i.e. 158) to letters, provides some keyboard like US, French...
extern crate pic8259; //TODO: Switch from PIC (Programmable interupt controller) to APIC (Advanced PIC)
extern crate spin; // Mutex and lock
extern crate uart_16550; // Helps us to talk to QEMU (serial print and shutdown)
extern crate x86_64; // A lot of asm macros, and useful for paging... Everything far and near CPU related // Reimplementation of HashMap and HashSet from stdlib
                     //TODO Make vga cursor move (os dev vga terminal doc) so we don't need to do our blinking, which means we don't need hashmaps anymore
extern crate bitfield;
