use crate::{
    gdt, interrupts, allocator,
    memory::{self, BootInfoFrameAllocator, MemoryHandler},
    task::executor::Executor,
};

use x86_64::{VirtAddr, structures::paging::OffsetPageTable};
use bootloader::BootInfo;
// Supress compiler warning about unused imports, but if removed, error
#[allow(unused_imports)]
use crate::println;

pub fn init(boot_info: &'static BootInfo) -> () {
    gdt::init();
    interrupts::init_idt(); // Init the interrupt descriptor table
    unsafe { interrupts::PICS.lock().initialize() };
    x86_64::instructions::interrupts::enable(); // Enable hardware interrupts

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset); // Get the physical memory offset
    #[allow(const_item_mutation)]
    crate::memory::HANDLER.init(phys_mem_offset, &boot_info.memory_map);

    allocator::init_heap().expect("heap initialization failed"); // Initialize the heap allocator
}

pub fn end() -> ! {
    let mut executor = Executor::new();
    // executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}