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


use defmt::dbg;
use defmt::info;
use x86_64::VirtAddr;

pub mod state;      pub use state::Kernel;
pub mod terminal;   pub use terminal::writer;pub use terminal::prompt;
pub mod interrupts; pub use interrupts::timer;
pub mod gdt;        
pub mod memory;     pub use crate::memory::handler::MemoryHandler;
pub mod allocator;  
pub mod task;       
pub mod test;       pub use test::{exit_qemu, QemuExitCode, test_panic_handler};
pub mod boot;       pub use boot::{hlt_loop,boot};
pub mod cpu;        
pub mod pci;        
pub mod apic;       

//-----------TESTS HANDLING-----------
#[cfg(test)]
use core::panic::PanicInfo;
#[cfg(test)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {test_panic_handler(info)}

#[cfg(test)]
use bootloader::{entry_point, BootInfo};
#[cfg(test)]
entry_point!(test_kernel_main);
#[cfg(test)]
fn test_kernel_main(boot_info: &'static BootInfo) -> ! {
    init(boot_info);
    test_main();
    hlt_loop()
}

//TODO: Remove the need for these

extern crate crossbeam_queue;
extern crate futures_util; // Async/Await, not very used for now
extern crate conquer_once;
extern crate x86_64; // A lot of asm macros, and useful for paging... Everything far and near CPU related
extern crate alloc; // Lib which stores some useful structs on the heap / smart pointers from stdlib like Vec, String, Box...
extern crate lazy_static; // Useful to declare some static without using only 'const fn ...'
extern crate spin; // Mutex and lock
extern crate volatile; //TODO: replace by core::Volatile... In vga_buffer in terminal
extern crate uart_16550;
extern crate pic8259; //TODO: Switch from PIC (Programmable interupt controller) to APIC (Advanced PIC)
extern crate pc_keyboard; // Transforms keyboard scancode (i.e. 158) to letters, provides some keyboard like US, French...
extern crate bootloader; // The bootloader crate, usefull for boot_info, paging and other stuff
extern crate linked_list_allocator;
extern crate hashbrown; // Reimplementation of HashMap and HashSet from stdlib