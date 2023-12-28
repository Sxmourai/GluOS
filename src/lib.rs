#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(test::runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(linkage)]
#![feature(naked_functions)]
#![allow(unused, dead_code)] //TODO: Only for debug (#[cfg(debug_assertions)])
#![feature(int_roundings)] // To use div_ceil and other utilities... Not in stable because breaks num-bigint crate, but fixed now, no worries =)

use bootloader::{entry_point, BootInfo};

pub mod state;
pub mod drivers;
pub mod test;
pub mod boot;
pub mod bit_manipulation;
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
extern crate conquer_once;
extern crate crossbeam_queue;
extern crate futures_util; // Async/Await, not very used for now
extern crate hashbrown;
extern crate lazy_static; // Useful to declare some static without using only 'const fn ...'
extern crate linked_list_allocator;
extern crate pc_keyboard; // Transforms keyboard scancode (i.e. 158) to letters, provides some keyboard like US, French...
extern crate pic8259; //TODO: Switch from PIC (Programmable interupt controller) to APIC (Advanced PIC)
extern crate spin; // Mutex and lock
extern crate uart_16550;
extern crate volatile; //TODO: replace by core::Volatile... In vga_buffer in terminal
extern crate x86_64; // A lot of asm macros, and useful for paging... Everything far and near CPU related // Reimplementation of HashMap and HashSet from stdlib
                     //TODO Make vga cursor move (os dev vga terminal doc) so we don't need to do our blinking, which means we don't need hashmaps anymore
