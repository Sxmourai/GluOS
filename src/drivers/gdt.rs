use lazy_static::lazy_static;
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
    GDT.0.load();
    unsafe {
        SS::set_reg(SegmentSelector(0));
        DS::set_reg(SegmentSelector(0));
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
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
        //TODO tss.privilege_stack_table
        tss
    };
}
lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        gdt.add_entry(Descriptor::user_code_segment());
        gdt.add_entry(Descriptor::user_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

pub struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}
