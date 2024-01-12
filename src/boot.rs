use crate::{
    drivers::{
        self,
        disk::ata::{Channel, DiskLoc, Drive},
        fs::fs_driver::FsDriver,
        memory::handler::MemoryHandler,
    },
    serial_println,
    state::get_state,
    task::executor::Executor,
    user::{self, shell::Shell},
};

pub fn boot(boot_info: &'static bootloader::BootInfo) {
    //TODO Can't use vecs, Strings before heap init (in memoryHandler init), which means we can't do trace... Use a constant-size list ?
    drivers::gdt::init();
    let mem_handler = MemoryHandler::init_heap_and_frame_allocator(
        boot_info.physical_memory_offset,
        &boot_info.memory_map,
    );
    drivers::interrupts::init();
    user::log::initialize_logger();
    drivers::disk::ata::init();
    drivers::time::init();
    drivers::video::init_graphics();
    let fs_driver = FsDriver::new(DiskLoc(Channel::Primary, Drive::Slave));
    get_state().init(boot_info, mem_handler, fs_driver);
    serial_println!("\t[Done booting]\n");
    // get_state().fs().lock().write_dir("hello");
    Shell::new();
}

pub fn end() -> ! {
    //TODO Implement async stuff & all in Executor
    // hlt_loop()
    let mut executor = Executor::new();
    // // executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run() // Replaces halt loop
}

pub fn hlt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
