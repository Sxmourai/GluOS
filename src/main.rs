#![no_std]
#![no_main]
#![allow(unused)]
#![feature(custom_test_frameworks)]
#![test_runner(kernel::test::runner)]
#![reexport_test_harness_main = "test_runner"]

extern crate alloc;
extern crate bootloader;
extern crate kernel;
extern crate x86_64;

use crate::kernel::{hlt_loop, serial_println};
use alloc::vec::Vec;
use bootloader::{entry_point, BootInfo};
use core::{
    ffi::{c_uchar, c_ushort},
    panic::PanicInfo,
};
use hashbrown::HashMap;
use kernel::{
    println,
    prompt::input,
    serial_print,
    state::get_mem_handler,
    terminal::{
        console::{pretty_print, CONSOLE},
        shell::Shell,
    },
    writer::{inb, outb, outb16, inw}, pci::pci_data::print_all_pci_devices, is_bit_set, memory::read_phys_memory_and_map, serial_print_all_bits, err, log::{self, print_trace},
};
use pci_ids::SubSystem;
use x86_64::{instructions::hlt, VirtAddr};

// Sets the entry point of our kernel for the bootloader. This means we can have the 'boot_info' variable which stores some crucial info
entry_point!(kernel_main);
// Main function of our kernel (1 func to start when boot if not in test mode). Never returns, because kernel runs until machine poweroff
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    kernel::boot(boot_info);

    kernel::pci::ata::init();

    // let identify_data = unsafe {
    //     // Wait for the drive to be ready (BSY = 0, DRQ = 1)

    //     // Send the IDENTIFY DEVICE command (0xEC) to the Command Register (0x1F7)
    //     serial_println!("here it's working");
        // outb(0x177, 0xEC);

    //     // Wait for the drive to be ready (BSY = 0, DRQ = 1)
    //     while (inb(0x177) & 0xC0) != 0x40 {}

    //     // Read the 512 bytes of data from the Data Register (0x1F0-0x1F7)
    //     let mut data = [0u16; 256];
    //     for i in 0..256 {
    //         data[i] = inw(0x170);
    //     }

    //     data
    // };
    // println!("Model Number: {:04X}", identify_data[0]);
    // serial_println!("{:?}", identify_data);

    for device in kernel::pci::pci_device_iter() {
        if device.class == 1 && device.subclass == 1 {
            let pi_reg = device.pci_read_32(9) as u8;
                // Check the primary channel (bit 0) of the programming interface.
            let primary_channel_supported = pi_reg & 0x01 == 0;

            // Check the secondary channel (bit 1) of the programming interface.
            let secondary_channel_supported = pi_reg & 0x02 == 0;

            // Check bit 7 to determine if ATAPI is supported.
            let atapi_supported = pi_reg & 0x80 != 0;
            let r = device.prog_if;
            serial_println!("{:?} {:b} {:b}", (primary_channel_supported, secondary_channel_supported, atapi_supported), r, pi_reg);
            serial_println!("AA{:?}", device.pci_read_32(0x20));
            let mode = if is_bit_set(r, 7) { "NATIVE" } else { "COMPATIBILITY" };
            let can_modify_bit_0 = is_bit_set(r, 6);
            let second_chan_mode = if is_bit_set(r, 5) { "NATIVE" } else { "COMPATIBILITY" };
            let can_modify_bit_2 = is_bit_set(r, 4);
            let dma_support = is_bit_set(r, 0); //When set, this is a bus master IDE controller
            serial_println!("Mode: {}\tCan modify prim chan mode: {}\tSecond channel mode: {}\tCan modify second chan mode: {}\tDMA Supported: {}", mode, can_modify_bit_0, second_chan_mode, can_modify_bit_2, dma_support);
            serial_println!("{:?}", device.bars);
            // Assuming bar4_value contains the non-zero value from BAR4
            let primary_port = (device.bars[4] & 0xFFFC) as u16; // Extract the primary I/O port address
            let secondary_port = ((device.bars[4] >> 16) & 0xFFFC) as u16; // Extract the secondary I/O port address

            println!("Primary Port: 0x{:X}", primary_port);

            unsafe { outb(primary_port, 0xEC) };



            println!("Secondary Port: 0x{:X}", secondary_port);

            for (i,bar) in device.bars.iter().enumerate() {
                if *bar != 0 {
                    // Check if the BAR represents a memory-mapped address
                    if bar & 0x1 == 0 {
                        // Memory-mapped address
                        let memory_address = bar & !0x3; // Mask out the two least significant bits
                        // Read the size of the memory region from the device-specific register
                        // For example, if it's a 32-bit BAR, the size will be 4 bytes (1 << 2).
                        let memory_size = 1 << bar; // Replace `2` with the actual size of the BAR (depends on the device).

                        // Use `memory_address` and `memory_size` to access the memory-mapped registers of the IDE controller.
                        let p = device.determine_mem_base(i);
                        let q = device.determine_mem_size(i);
                        serial_println!("Memory: {:?}", (memory_address, memory_size));
                        serial_println!("Memory lib: {:?}", (p,q));
                    } else {
                        // I/O port address
                        // unsafe { outb((*bar).try_into().unwrap(), u8::MAX) };
                        // hlt();
                        let bar_size = unsafe { inb((*bar).try_into().unwrap()) }; // Send all ones
                        let io_port_address = bar & !0x1; // Mask out the least significant bit
                        // Read the size of the I/O port region from the device-specific register
                        // For example, if it's a 16-bit BAR, the size will be 2 bytes (1 << 1).
                        let io_port_size = 1 << bar_size; // Replace `1` with the actual size of the BAR (depends on the device).
                        // Use `io_port_address` and `io_port_size` to access the I/O ports of the IDE controller.
                        
                        serial_println!("Size: {:b}\tAddress: {:b}\tPort size: {}", bar_size, io_port_address, io_port_size);
                    }
                }
            }
        }
    }
    // kernel::pci::ata::initialize_sata_controller();


    // unsafe { kernel::pci::ata::initialize_sata_controller() };
    // Shell::new();
    serial_println!("Done booting !");
    hlt_loop()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    err!("Error: {}", info);
    print_trace();
    hlt_loop()
}
