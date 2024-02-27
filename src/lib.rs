#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(test::runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(linkage)]
#![feature(naked_functions)]
#![feature(alloc_error_handler)]
#![cfg_attr(debug_assertions, allow(unused))]
#![allow(dead_code)]
#![deny(unsafe_op_in_unsafe_fn)]
// Clippy config
// #![deny(clippy::all)]
#![deny(clippy::correctness)]
#![deny(clippy::suspicious)]
#![deny(clippy::perf)]
#![warn(clippy::pedantic)]
#![warn(clippy::complexity)]
#![cfg_attr(not(debug_assertions), deny(clippy::unwrap_used))]
#![cfg_attr(not(debug_assertions), deny(clippy::expect_used))]

pub mod bit_manipulation;
pub mod boot;
pub mod drivers;
pub mod state;
pub mod sync;
pub mod test;
pub mod user;

pub use drivers::*;
pub use user::*;

#[cfg(test)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    test::panic_handler(info)
}

extern crate alloc; // Lib which stores some useful structs on the heap / smart pointers from stdlib like Vec, String, Box...
extern crate bitfield;
extern crate bootloader; // The bootloader crate, usefull for boot_info, paging and other stuff
extern crate hashbrown;
extern crate lazy_static; // Useful to declare some static without using only 'const fn ...'
extern crate pc_keyboard; // Transforms keyboard scancode (i.e. 158) to letters, provides some keyboard like US, French...
extern crate pic8259; //TODO: Switch from PIC (Programmable interupt controller) to APIC (Advanced PIC)
extern crate spin; // Mutex, other sync primitives...
extern crate uart_16550; // Helps us to talk to QEMU (serial print and shutdown)
extern crate x86_64; // A lot of asm macros, and useful for paging... Everything far and near CPU related // Reimplementation of HashMap and HashSet from stdlib
