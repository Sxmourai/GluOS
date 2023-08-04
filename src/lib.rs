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

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use x86_64::VirtAddr;

pub mod state;
pub use state::Kernel;
pub mod terminal;
pub use terminal::prompt;
pub use terminal::writer;
pub mod interrupts;
pub use interrupts::timer;
pub mod gdt;
pub mod memory;
use crate::log::print_trace;
pub use crate::memory::handler::MemoryHandler;
pub mod allocator;
pub mod task;
pub mod test;
pub use test::{exit_qemu, QemuExitCode};
pub mod boot;
pub use boot::{boot, hlt_loop};
pub mod cpu;
pub mod pci;
pub mod log;
pub mod fs;
//pub mod apic; //!causes compiler error


//-----------TESTS HANDLING-----------
use core::panic::PanicInfo;
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    test::panic_handler(info)
}

#[cfg(test)]
use bootloader::{entry_point, BootInfo};
#[cfg(test)]
entry_point!(test_kernel_main);
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    boot(boot_info);
    test_main();
    test::end()
}

// ! UTILITIES FUNCTIONS
pub fn find_string(bytes: &[u8], search_string: &[u8]) -> Option<usize> {
    let search_len = search_string.len();

    for i in 0..(bytes.len() - search_len + 1) {
        if &bytes[i..(i + search_len)] == search_string {
            return Some(i);
        }
    }

    None
}

pub fn serial_print_all_bits<T: Into<u128>>(num: T) {
    let num = num.into();
    let size = core::mem::size_of::<T>() * 8;

    for i in (0..size).rev() {
        let bit = (num >> i) & 1;
        serial_print!("{}", bit);
    }
    serial_print!(" - ");
}

pub fn bytes<T: Into<u128>>(num: T) -> String {
    let mut result = String::new();
    let num = num.into();
    let size = core::mem::size_of::<T>() * 8;

    for i in (0..size).rev() {
        let bit = (num >> i) & 1;
        let str_bit = if bit == 0 {'0'} else {'1'};
        result.push(str_bit);
    }
    result
}
pub fn numeric_to_char_vec<T>(value: T) -> String
where
    T: Into<u64>,
{
    let value_u64 = value.into();
    let mut char_vec = String::new();

    for shift in (0..(core::mem::size_of::<T>() * 8)).step_by(8) {
        let byte: u8 = ((value_u64 >> shift) & 0xFF) as u8;
        char_vec.push(char::from(byte));
    }

    char_vec
}
pub fn bytes_to_numeric<T>(bytes: &[u8]) -> T
where
    T: From<u64>,
{
    let mut result: u64 = 0;

    for (i, &byte) in bytes.iter().enumerate() {
        if i < core::mem::size_of::<T>() {
            result |= (byte as u64) << (i * 8);
        }
    }

    T::from(result)
}


// pub unsafe fn from_raw_parts_unchecked<T>(ptr:*mut T, len:usize) -> Vec<T>
//     where T: Copy  {
//     let mut v = Vec::new();
//     let ele_size = core::mem::size_of::<T>();
//     for i in 0..len {
//         let addr = ptr as usize+i*ele_size;
//         v.push(*ptr as T);
//     }
//     v
// }


pub fn is_bit_set(byte: u8, bit_position: u8) -> bool {
    // Create a mask with only the bit at the specified position set to 1
    let mask = 1 << bit_position;
    // Perform a bitwise AND operation with the mask
    // If the result is non-zero, the bit is set to 1; otherwise, the bit is 0.
    byte & mask != 0
}

fn u8_to_u32(u8_data: &[u8]) -> Vec<u32> {
    let mut u32_data = Vec::new();

    for i in (0..u8_data.len()).step_by(4) {
        let mut sum = 0;
        for &byte in &u8_data[i..i + 4] {
            // Perform the conversion by combining four consecutive u8 values into a u32
            sum = (sum << 8) | u32::from(byte);
        }
        u32_data.push(sum);
    }

    u32_data
}
pub fn u8_bytes_to_u32(bytes: &[u8]) -> u32 { //TODO Change u32 to T
    let mut result = 0u32;
    for (i, &byte) in bytes.iter().rev().enumerate() {
        if i > core::mem::size_of::<u32>() {break}
        result = (result << 8) | u32::from(byte);
    }

    result
}
// trait U16sTo {}
// impl U16to for u32 {}
// impl U16to for u64 {}
pub fn u16_bytes_to_u32(bytes: &[u16]) -> u32 { //TODO Change u32 to T
    let mut result = 0;
    for (i, &byte) in bytes.iter().rev().enumerate() {
        if i > core::mem::size_of::<u32>() {break}
        result = (result << core::mem::size_of::<u16>()) | u32::from(byte);
    }
    result
}
pub fn u16_bytes_to_u64(bytes: &[u16]) -> u64 {
    let mut result = 0;
    for (i, &byte) in bytes.iter().rev().enumerate() {
        if i > core::mem::size_of::<u64>() {break}
        result = (result << core::mem::size_of::<u16>()) | u64::from(byte);
    }
    result
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
