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


use core::fmt::Display;
use core::mem::size_of;
use core::ops::BitOr;
use core::ops::Shl;
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

///! DANGER ZONE DONT GO THERE 🤣
pub fn list_to_num<T,R>(mut content: impl Iterator<Item = T> + DoubleEndedIterator) -> R 
where T: Into<R>,
      R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default{
  let mut result = R::default();
  for (i, byte) in content.into_iter().rev().enumerate() {
      if i >= size_of::<R>()/size_of::<T>() {break}
      result = Into::<R>::into((result << size_of::<T>()*8)) | byte.into();
  }
  result
}
pub fn ptrlist_to_num<'a, T,R>(mut content: &mut (impl Iterator<Item = &'a T> + ?Sized + DoubleEndedIterator)) -> R 
where T: Into<R> + 'a + Clone,
      R: BitOr<Output = R> + Shl<usize> + From<<R as Shl<usize>>::Output> + Default{
  let mut result = R::default();
  for (i, byte) in content.into_iter().rev().enumerate() {
      if i >= size_of::<R>()/size_of::<T>() {break}
      result = Into::<R>::into((result << size_of::<T>()*8)) | Into::<R>::into(byte.clone());
  }
  result
}

pub fn u16_to_u8(w: u16) -> (u8, u8) {
    (((w >> 8) as u8), (w & 0xFF) as u8)
}
struct CharArray<const N: usize> ([char; N]);

impl<const N: usize> Display for CharArray<N> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}//TODO implement debugging
struct CharSlice ([char]);

impl Display for CharSlice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in &self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}//TODO implement debugging
struct CharSlicePtr<'a> (&'a [char]);

impl Display for CharSlicePtr<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut s = String::new();
        for element in self.0 {
            s.push(*element);
        }
        write!(f, "[{}]", s)
    }
}//TODO implement debugging



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
