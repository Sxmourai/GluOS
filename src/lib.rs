#![no_std]
#![cfg_attr(test, no_main)]
#![feature(custom_test_frameworks)]
#![test_runner(test::runner)]
#![reexport_test_harness_main = "test_main"]
#![feature(abi_x86_interrupt)]
#![feature(const_mut_refs)]
#![feature(linkage)]
// #![feature(llvm_asm)]
#![feature(naked_functions)]


pub mod state;      pub use state::Kernel;
pub mod serial;     
pub mod terminal;   pub use terminal::writer;pub use terminal::prompt;
pub mod interrupts; 
pub mod gdt;        
pub mod memory;     
pub mod allocator;  
pub mod task;       
pub mod test;       pub use test::{exit_qemu, QemuExitCode, test_panic_handler};
pub mod boot;       pub use boot::hlt_loop;
pub mod timer;      
pub mod cpu;        
pub mod pci;        
pub mod apic;       


pub fn init(boot_info: &'static bootloader::BootInfo) { state::STATE.lock().init(boot_info) }






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

//TODO: Adding extern crates... Needs to be removed but idk how

extern crate crossbeam_queue;
extern crate futures_util;
extern crate conquer_once;
extern crate x86_64;
extern crate alloc;
extern crate lazy_static;
extern crate spin;
extern crate volatile;
extern crate uart_16550;
extern crate pic8259;
extern crate pc_keyboard;
extern crate bootloader;
extern crate linked_list_allocator;
extern crate hashbrown;