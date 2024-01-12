//TODO: Implement proper paging & all
//TODO: Make a "simple" function to map ANY frame to a new page. Need this to access rsdp pointer
// https://os.phil-opp.com/paging-implementation/#using-offsetpagetable




use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::structures::paging::PageTableFlags;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

pub mod acpi;
pub mod allocator;
pub mod apic;
pub mod frame_allocator;
pub mod handler;
pub mod rsdp;

pub use frame_allocator::BootInfoFrameAllocator;




use self::handler::MemoryHandler;

// #[alloc_error_handler]
// pub fn alloc_error(size: usize, align: usize) -> ! {

// }

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

    unsafe { &mut *page_table_ptr }
}

/// Initialize a new OffsetPageTable.
///
/// This function is unsafe because the caller must guarantee that the
/// complete physical memory is mapped to virtual memory at the passed
/// `physical_memory_offset`. Also, this function must be only called once
/// to avoid aliasing `&mut` references (which is undefined behavior).
// pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
//     let level_4_table = active_level_4_table(physical_memory_offset);
//     OffsetPageTable::new(level_4_table, physical_memory_offset)
// }

// end_page is using .containing address
//TODO Map a page when a page fault occurs (in interrupts/exceptions.rs)
pub unsafe fn read_phys_memory_and_map(
    mem_handler: &mut MemoryHandler,
    location: u64,
    size: usize,
    end_page: u64,
) -> &'static [u8] {
    let flags: PageTableFlags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE; // TODO Change this to a constant

    let _size_64 = size as u64;
    let start_frame_addr = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(location))
        .start_address()
        .as_u64();
    let mut offset = 0;
    for i in (start_frame_addr..start_frame_addr + size as u64).step_by(4096) {
        // Map all frames that might be used
        let page: Page<Size4KiB> = Page::containing_address(VirtAddr::new(end_page + offset));
        let phys_frame = PhysFrame::containing_address(PhysAddr::new(i));

        // serial_println!("Physical frame_adress: {:X}\t-\tLocation: {:X}
        // Computed location {:X}\t-\tFrame to page: {:X} (Provided (unaligned): {:X})
        // Currently mapping: Physical({:X}-{:X})\t-\tVirtual({:X}-{:X})
        // ", phys_frame.start_address().as_u64(), location, end_page, page.start_address().as_u64(),end_page, i,i+4096, end_page+offset, end_page+offset+4096);
        mem_handler.map_to(page, phys_frame, flags);
        offset += 4096;
    }
    // Reads the content from memory, should be safe
    let end_page_start_addr = Page::<Size4KiB>::containing_address(VirtAddr::new(end_page))
        .start_address()
        .as_u64();
    let phys_offset = location
        - PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(location))
            .start_address()
            .as_u64();
    let start_addr = phys_offset + end_page_start_addr;
    unsafe { core::slice::from_raw_parts(start_addr as *const u8, size) }
}
