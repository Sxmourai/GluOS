use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::structures::paging::PageTableFlags as Flags;
use x86_64::{
    structures::paging::{FrameAllocator, Mapper, Page, PhysFrame, Size4KiB},
    PhysAddr, VirtAddr,
};

use crate::mem_handler;

/// A FrameAllocator that returns usable frames from the bootloader's memory map.
#[derive(Debug, Clone)]
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// Create a FrameAllocator from the passed memory map.
    ///
    /// # Safety
    /// This function is unsafe because the caller must guarantee that the passed
    /// memory map is valid. The main requirement is that all frames that are marked
    /// as `USABLE` in it are really unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        return BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
    /// Returns an iterator over the usable frames specified in the memory map.
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // get usable regions from memory map
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| return r.region_type == MemoryRegionType::Usable);
        // map each region to its address range
        let addr_ranges = usable_regions.map(|r| return r.range.start_addr()..r.range.end_addr());
        // transform to an iterator of frame start addresses
        let frame_addresses = addr_ranges.flat_map(|r| return r.step_by(4096));
        // create `PhysFrame` types from the start addresses
        frame_addresses.map(|addr| return PhysFrame::containing_address(PhysAddr::new(addr)))
    }

    // pub unsafe fn map_physical_region(
    //     &mut self,
    //     physical_address: usize,
    // ) -> x86_64::structures::paging::Page {
    //     let frame =
    //         PhysFrame::from_start_address(PhysAddr::new(physical_address.try_into().unwrap()))
    //             .unwrap();
    //     let flags = Flags::PRESENT | Flags::WRITABLE;

    //     let page = Page::containing_address(VirtAddr::new(0xfffffff9));

    //     let mut mem_handler = unsafe {mem_handler!()};
    //     let map_to_result = unsafe {
    //         // FIXME: this is not safe, we do it only for testing
    //         mem_handler.mapper.map_to(page, frame, flags)
    //     };
    //     page
    // }
}
// impl AcpiHandler for BootInfoFrameAllocator {
//     unsafe fn map_physical_region<T>(&self, physical_address: usize, size: usize) -> rsdp::handler::PhysicalMapping<Self, T> {
//         let phys_mem_offset = crate::state::get_boot_info().physical_memory_offset as usize;
//         let frame_allocator = crate::state::get_mem_handler().get_mut().frame_allocator;
//         let ptr = core::ptr::NonNull::new(&mut T).unwrap();

//         rsdp::handler::PhysicalMapping::<BootInfoFrameAllocator, usize>::new(physical_address, ptr, 4096, 4096, frame_allocator)
//     }

//     fn unmap_physical_region<T>(region: &rsdp::handler::PhysicalMapping<Self, T>) {panic!()}
// }

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        return frame
    }
}
