//TODO: Implement proper paging & all
//TODO: Make a "simple" function to map ANY frame to a new page. Need this to access rsdp pointer
// https://os.phil-opp.com/paging-implementation/#using-offsetpagetable

use core::ops::Range;

use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use x86_64::structures::paging::PageTableFlags;

pub mod frame_allocator;
pub mod handler;
pub mod rsdp;
pub mod apic;

pub use frame_allocator::BootInfoFrameAllocator;

use crate::serial_println;

/// Creates an example mapping for the given page to frame `0xb8000`.
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        // FIXME: this is not safe, we do it only for testing
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}

/// Returns a mutable reference to the active level 4 table.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr // unsafe
}

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

// end_page is using .containing address
//TODO Make a loop to map all frames that user is trying to get access:
//i.e. size = 4096, so location is at a certain frame, but the end location is in another frame that ISNT MAP, which causes page fault
// Two ways to fix: 
// 1. Worst, just make a loop, align etc.
// 2. Map a page when a page fault occurs (refer to interrupts/exceptions)
pub fn read_phys_memory_and_map(location: u64, size: usize, end_page:u64) -> &'static [u8] {
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    let phys_frame: PhysFrame<Size4KiB> = PhysFrame::containing_address(PhysAddr::new(location));
    let mut mem_handler = unsafe { crate::state::STATE.get_mem_handler() };
    let mut mem_h = mem_handler.get_mut();
    let page = Page::containing_address(VirtAddr::new(end_page));
    unsafe { mem_h.mapper.map_to(page, phys_frame, flags, &mut mem_h.frame_allocator).unwrap().flush() };

    let addr = location-phys_frame.start_address().as_u64() + page.start_address().as_u64();
    
    // serial_println!("Physical frame_adress: {:x}\t-\tLocation: {:x}\nComputed location {:x}\t-\tFrame to page: {:x} (Provided (unaligned): {:x})", phys_frame.start_address().as_u64(), location, addr, page.start_address().as_u64(),end_page);
    unsafe { read_memory(addr as *const u8, size) }
}
// Create a slice from the memory location with the given size
pub unsafe fn read_memory(location: *const u8, size: usize) -> &'static [u8] {
    core::slice::from_raw_parts(location, size)    
}
