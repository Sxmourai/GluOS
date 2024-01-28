use core::sync::atomic::AtomicU64;

use x86_64::{structures::paging::{Page, PageTableFlags, PhysFrame, Size4KiB}, PhysAddr, VirtAddr};

use crate::{boot::hlt_loop, dbg, descriptor_tables, gdt::KERNEL_STACK_SIZE, interrupts::apic::ipi::InterruptCommand, mem_handler, memory::handler::{map, map_frame}, pit::udelay};

const CODE: &[u8] = include_bytes!("ap.bin");
#[allow(clippy::declare_interior_mutable_const)]
pub const AP_STACKS_ADDR: usize = 109<<39;
#[allow(clippy::declare_interior_mutable_const)]
pub const AP_CORE_COUNTER: AtomicU64 = AtomicU64::new(0);
#[allow(clippy::declare_interior_mutable_const)]
pub const AP_CORE_COUNTER_DONE: AtomicU64 = AtomicU64::new(0);

/// Tries to init the cores
pub fn init() {
    let num_core = descriptor_tables!().num_core();
    if num_core==1 {
        // Only one core, so no other cores to start up
        return
    }
    allocate_stacks(num_core);
    // SAFETY WARNING null ptr mapped: dereferencing a null ptr is now allowed
    map_frame(
        Page::<Size4KiB>::from_start_address(VirtAddr::new(0)).unwrap(),
        PhysFrame::from_start_address(PhysAddr::new(0)).unwrap(),
        PageTableFlags::PRESENT | PageTableFlags::WRITABLE,
    );
    
    dbg!("b");
    dbg!(CODE.as_ptr());
    
    let lvl4table = mem_handler!().mapper.level_4_table();
    dbg!("a");
    let lvl4table_addr = core::ptr::addr_of!(lvl4table) as usize;

    let core_counter_addr = AP_CORE_COUNTER.as_ptr() as usize;

    dbg!("a");
    let stack_stride = calc_stack_stride();

    dbg!("a");
    let stack_base = AP_STACKS_ADDR + stack_stride - 1;

    let entry_function_addr = (ap_entry_fn as *const ());
    unsafe {
        core::ptr::copy_nonoverlapping(CODE.as_ptr(), core::ptr::NonNull::dangling().as_ptr(), CODE.len());

        *(0x0A00 as *mut usize) = lvl4table_addr;
        *(0x0B00 as *mut usize) = core_counter_addr;
        *(0x0B10 as *mut usize) = stack_stride;
        *(0x0B20 as *mut usize) = stack_base;
        *(0x0B30 as *mut usize) = entry_function_addr as usize;
    }

    startup_aps();
    
    let mut time_out = 1_000_000;

    while AP_CORE_COUNTER_DONE.load(core::sync::atomic::Ordering::Acquire) < num_core as u64 && time_out > 0 {
        core::hint::spin_loop();
        time_out -= 1;
    }

    {
        let mut mem = mem_handler!();
        unsafe { mem.unmap(Page::<Size4KiB>::from_start_address(VirtAddr::new(0)).unwrap()) }.unwrap();
    }

    if time_out == 0 {
        panic!("AP startup timed out");
    } else {
        log::info!("All aps started");
    }
}

fn startup_aps() {
    // let numcores = unsafe {descriptor_tables!().num_core()} as u32;
    // let lapic_ids = unsafe {&descriptor_tables!().madt.cores};
    // let l = lapic_addr; // Makes an alias to call functions and take less place
    // copy the AP trampoline code to a fixed address in low conventional memory (to address 0x0800:0x0000)

    // for i in 0..numcores {
    //     // do not start BSP, that's already running this code
    //     if lapic_ids[i as usize].0 == bsp_id {continue}
    //     // --------------send INIT IPI------------
    //     // clear APIC errors
    //     unsafe{write_lapic(l, 0x280, 0)}
    //     select_ap(l, i);
    //     // trigger INIT IPI
    //     unsafe{write_lapic(l,0x300, (read_lapic(l,0x300) & 0xfff00000) | 0x00C500)}
    //     wait_for_delivery(l);        
    //     select_ap(l, i);
    //     // deassert
    //     unsafe{write_lapic(l, 0x300, (read_lapic(l, 0x300) & 0xfff00000) | 0x008500)}
    //     wait_for_delivery(l);
    //     udelay(10_000);
    // 	// send STARTUP IPI (twice)
    //     for j in 0..2 {
    //         // clear APIC errors
    //         unsafe { write_lapic(l, 0x280, 0) };
    //         select_ap(l, i);
    //         // trigger STARTUP IPI for 0800:0000
    //         unsafe{write_lapic(l,0x300, (read_lapic(l, 0x300) & 0xfff0f800) | 0x000608)}
    //         udelay(200);
    //         wait_for_delivery(l);
    //     }
    // }
    //
    let mut init_cmd = InterruptCommand(0);
    init_cmd.set_interupt_vector(0);
    init_cmd.set_delivery_mode(5);
    init_cmd.set_destination_mode_logical(false);
    init_cmd.set_de_assert(false);
    init_cmd.set_not_de_assert(true);
    init_cmd.set_destination_type(3);
    let mut startup_cmd = InterruptCommand(0);
    startup_cmd.set_interupt_vector(0);
    startup_cmd.set_delivery_mode(6);
    startup_cmd.set_destination_mode_logical(false);
    startup_cmd.set_de_assert(false);
    startup_cmd.set_not_de_assert(true);
    startup_cmd.set_destination_type(3);
    
    let mut apic = crate::interrupts::apic::get_apic();
    apic.write_interrupt_command(init_cmd);
    crate::pit::udelay(10_000);
    apic.write_interrupt_command(startup_cmd);
    crate::pit::udelay(200);
    apic.write_interrupt_command(startup_cmd);
    // crate::interrupts::apic::write_interrupt_command(unsafe { &mut lapic }, init_cmd);
    // crate::pit::udelay(10_000).unwrap();
    // crate::interrupts::apic::write_interrupt_command(unsafe { &mut lapic }, startup_cmd);
    // crate::pit::udelay(200).unwrap();
    // crate::interrupts::apic::write_interrupt_command(unsafe { &mut lapic }, startup_cmd);
}
// // select AP
// fn select_ap(lapic_addr: u32, core:u32) {
//     unsafe{write_lapic(lapic_addr,0x310, (read_lapic(lapic_addr, 0x310) & 0x00ffffff) | ((core)<<24))}
// }
// fn wait_for_delivery(lapic_addr: u32) {
//     unsafe {
//         core::arch::asm!("pause", "");
//         while read_lapic(lapic_addr, 0x300) & (1<<12)!=0 {
//             core::arch::asm!("pause", "");
//         }
//     }
// }


unsafe extern "C" fn ap_entry_fn(ap_index: u64) -> ! {
    AP_CORE_COUNTER_DONE.fetch_add(1, core::sync::atomic::Ordering::AcqRel);

    log::info!(
        "Core started: index({})",
        ap_index + 1,
    );
    //TODO Proper booting
    hlt_loop()
}

fn allocate_stacks(num_core: usize) {
    let pages_per_core = KERNEL_STACK_SIZE.div_ceil(4096);
    let mut addr = AP_STACKS_ADDR;
    for _ in 0..num_core {
        addr += 4096; // add guard page
        for _ in 0..pages_per_core {
            dbg!(addr);
            let page = Page::<Size4KiB>::from_start_address(VirtAddr::new(addr as u64)).unwrap();
            map(page, PageTableFlags::WRITABLE | PageTableFlags::NO_EXECUTE);
            addr += 4096;
        }
    }
}
const fn calc_stack_stride() -> usize {
    let pages_per_core = KERNEL_STACK_SIZE.div_ceil(4096);
    (pages_per_core + 1) * 4096
}