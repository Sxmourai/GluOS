use crate::{
    gdt, interrupts, allocator,
    memory::{self, BootInfoFrameAllocator},
    task::{Task, keyboard, executor::Executor},
};

use x86_64::VirtAddr;
use bootloader::BootInfo;
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

pub fn init(boot_info: &'static BootInfo) {
    gdt::init();
    interrupts::init_idt(); // Init the interrupt descriptor table
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset); // Get the physical memory offset
    let mut mapper = unsafe { memory::init(phys_mem_offset) }; // Initialize the memory mapper
    let mut frame_allocator = unsafe {
        BootInfoFrameAllocator::init(&boot_info.memory_map) // Initialize the frame allocator
    };
    allocator::init_heap(&mut mapper, &mut frame_allocator)// Initialize the heap allocator
        .expect("heap initialization failed");
}

pub fn end() -> ! {
    let mut executor = Executor::new();
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}