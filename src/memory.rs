//TODO: Implement proper paging & all
//TODO: Make a "simple" function to map ANY frame to a new page. Need this to access rsdp pointer
// https://os.phil-opp.com/paging-implementation/#using-offsetpagetable
use core::borrow::BorrowMut;
use core::ops::Range;
use alloc::boxed::Box;
use alloc::sync::Arc;
use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use rsdp::Rsdp;
use rsdp::handler::AcpiHandler;
use spin::Mutex;
use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::structures::paging::mapper::PageTableFrameMapping;
use x86_64::{
    structures::paging::{
        FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};
use crate::serial_println;
// use crate::state::get_mem_handler;
// struct MyPageTableFrameMapping{next:usize}
// unsafe impl PageTableFrameMapping for MyPageTableFrameMapping {
//     fn frame_to_pointer(&self, frame: PhysFrame) -> *mut PageTable {
//         PageTable::new()
//     }
// }

#[derive(Debug)]
pub struct MemoryHandler {
    pub mapper: OffsetPageTable<'static>,
    pub frame_allocator: BootInfoFrameAllocator,
}
impl MemoryHandler {
    pub fn new(physical_memory_offset: VirtAddr, memory_map: &'static MemoryMap) -> Self {
            
        let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
        
        // let mut mapper = unsafe { x86_64::structures::paging::MappedPageTable::new(level_4_table, MyPageTableFrameMapping{next:0}) };
        let mut mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
        let mut frame_allocator = unsafe {
            BootInfoFrameAllocator::init(memory_map) // Initialize the frame allocator
        };
        crate::allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed"); // Initialize the heap allocator
        Self {
            mapper,
            frame_allocator
        }
    }

    // pub fn frame_allocator(&mut self) -> &mut BootInfoFrameAllocator {
    //     serial_println!("{:?}",self.frame_allocator);
    //     Arc::clone(self.frame_allocator.as_mut().unwrap())
    // }
    // pub fn mapper(&mut self) -> Arc<Mutex<OffsetPageTable<'static>>> {
    //     Arc::clone(self.mapper.as_mut().unwrap())
    // }
}

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

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
#[derive(Debug, Clone)]
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
    
    unsafe fn map_physical_region(&self, physical_address: usize) -> x86_64::structures::paging::Page {
        let frame =
            PhysFrame::from_start_address(PhysAddr::new(physical_address.try_into().unwrap())).unwrap();
        let flags = Flags::PRESENT | Flags::WRITABLE;

        let mut binding = crate::state::get_mem_handler();
        let mem_handler = binding.get_mut();

        let page =
            Page::containing_address(VirtAddr::new(0xfffffff9));

        let map_to_result = unsafe {
            // FIXME: this is not safe, we do it only for testing
            mem_handler.mapper.map_to(page, frame, flags, &mut mem_handler.frame_allocator)
        };
        map_to_result.expect("map_to failed").flush();
        page
    }
}
impl AcpiHandler for BootInfoFrameAllocator {
    unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> rsdp::handler::PhysicalMapping<Self, T> {
        let phys_mem_offset = crate::state::get_boot_info().physical_memory_offset as usize;
        let frame_allocator = crate::state::get_mem_handler().get_mut().frame_allocator;
        let ptr = core::ptr::NonNull::new(&mut T).unwrap();

        rsdp::handler::PhysicalMapping::<BootInfoFrameAllocator, usize>::new(physical_address, ptr, 4096, 4096, frame_allocator)
    }

    fn unmap_physical_region<T>(region: &rsdp::handler::PhysicalMapping<Self, T>) {panic!()}
}


unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next); //TODO: Store usable frames as a struct field
        self.next += 1;
        frame
    }
}

//////////////CODE FROM https://docs.rs/rsdp/latest/src/rsdp/lib.rs.html#175-203
// (rsdp crate)

/// Find the areas we should search for the RSDP in.
pub fn find_search_areas(frame_allocator: &BootInfoFrameAllocator) -> [Range<usize>; 2] {
    /*
     * Read the base address of the EBDA from its location in the BDA (BIOS Data Area). Not all BIOSs fill this out
     * unfortunately, so we might not get a sensible result. We shift it left 4, as it's a segment address.
     */
    // serial_println!("{:x}", unsafe { frame_allocator.map_physical_region(RSDP_BIOS_AREA_START) }.start_address());
    let ebda_start_mapping = unsafe { frame_allocator.map_physical_region(EBDA_START_SEGMENT_PTR) };
    let ebda_start = (unsafe { *ebda_start_mapping.start_address().as_ptr::<u16>() } as usize) << 4;
    [
        /*
         * The main BIOS area below 1MiB. In practice, from my [Restioson's] testing, the RSDP is more often here
         * than the EBDA. We also don't want to search the entire possible EBDA range, if we've failed to find it
         * from the BDA.
         */
        RSDP_BIOS_AREA_START..(RSDP_BIOS_AREA_END + 1),
        // Check if base segment ptr is in valid range for EBDA base
        if (EBDA_EARLIEST_START..EBDA_END).contains(&ebda_start) {
            // First KiB of EBDA
            ebda_start..ebda_start + 1024
        } else {
            // We don't know where the EBDA starts, so just search the largest possible EBDA
            EBDA_EARLIEST_START..(EBDA_END + 1)
        },
    ]
}

/// This (usually!) contains the base address of the EBDA (Extended Bios Data Area), shifted right by 4
const EBDA_START_SEGMENT_PTR: usize = 0x40e;
/// The earliest (lowest) memory address an EBDA (Extended Bios Data Area) can start
const EBDA_EARLIEST_START: usize = 0x80000;
/// The end of the EBDA (Extended Bios Data Area)
const EBDA_END: usize = 0x9ffff;
/// The start of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
const RSDP_BIOS_AREA_START: usize = 0xe0000;
/// The end of the main BIOS area below 1mb in which to search for the RSDP (Root System Description Pointer)
const RSDP_BIOS_AREA_END: usize = 0xfffff;
/// The RSDP (Root System Description Pointer)'s signature, "RSD PTR " (note trailing space)
pub const RSDP_SIGNATURE: &'static [u8; 8] = b"RSD PTR ";

pub fn search_for_on_bios(handler: &BootInfoFrameAllocator) -> Option<usize> {
    let mut rsdp_address = None;
    let areas = find_search_areas(handler);
    'areas: for area in areas.iter() {
        serial_println!("{:?}", area);
        // let mapping = unsafe { handler.map_physical_region(area.start) };

        for address in area.clone().step_by(16) {
            serial_println!("{:x}", address);
            let ptr_in_mapping = unsafe { *(address as *const isize) };
                // unsafe { area.start.as_ptr::<u8>().offset((address - area.start) as isize) };
            let signature = unsafe { *(ptr_in_mapping as *const [u8; 8]) };

            if signature == *RSDP_SIGNATURE {
                match unsafe { *(ptr_in_mapping as *const Rsdp) }.validate() {
                    Ok(()) => {
                        rsdp_address = Some(address);
                        break 'areas;
                    }
                    Err(err) => serial_println!("Invalid RSDP found at {:#x}: {:?}", address, err),
                }
            }
        }
    }
    rsdp_address
}