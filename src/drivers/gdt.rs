use core::cell::Cell;

use lazy_static::lazy_static;
use spin::RwLock;
use x86_64::instructions::segmentation::{Segment, CS};
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{DS, SS};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Initialises the GDT
/// See https://wiki.osdev.org/GDT for more infos
/// Also used by interrupt handler
/// See src/drivers/interrupts
pub fn init() {
    let mut gdt = GlobalDescriptorTable::new();
    let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
    let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
    // For userland
    let user_code_segment = gdt.add_entry(Descriptor::user_code_segment());
    let user_data_segment = gdt.add_entry(Descriptor::user_data_segment());
    unsafe {
        GDT.set(Some((
            gdt,
            Selectors {
                code_selector,
                tss_selector,
                user_code_segment,
                user_data_segment,
            },
        )))
    };
    let gdt = unsafe { GDT.get_mut().as_mut().unwrap_unchecked() };
    unsafe { gdt.0.load() };
    unsafe {
        SS::set_reg(SegmentSelector(0));
        CS::set_reg(gdt.1.code_selector);
        load_tss(gdt.1.tss_selector);
    }
}

pub const KERNEL_STACK_SIZE: usize = 4096 * 1024;
lazy_static! {
    pub static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { core::ptr::addr_of!(STACK) });
            stack_start + KERNEL_STACK_SIZE
        };
        tss.privilege_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut PRIV_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(unsafe { core::ptr::addr_of!(PRIV_STACK) });
            stack_start + KERNEL_STACK_SIZE
        };
        tss
    };
}
pub static mut GDT: Cell<Option<(GlobalDescriptorTable, Selectors)>> = Cell::new(None);
/// # Safety
/// Ensure gdt is initialised
pub fn get_gdt() -> &'static mut (GlobalDescriptorTable, Selectors) {
    unsafe { GDT.get_mut().as_mut().unwrap() }
}
#[derive(Clone)]
pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
    pub user_code_segment: SegmentSelector,
    pub user_data_segment: SegmentSelector,
}
