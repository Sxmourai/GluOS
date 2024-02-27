//! Used wiki.osdev.org/NVMe
//! Used mostly https://nvmexpress.org/wp-content/uploads/NVM-Express-Base-Specification-2.0d-2024.01.11-Ratified.pdf
//! https://github.com/doug65536/dgos/blob/master/kernel/device/nvme/nvme.cc
//! https://github.com/LemonOSProject/LemonOS/blob/master/Kernel/include/Storage/NVMe.h#L416
use core::ptr::addr_of;

use alloc::vec::Vec;
use bit_field::BitField;
use x86_64::{
    structures::paging::{Page, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use crate::{
    bit_manipulation::{all_zeroes, any_as_u8_slice},
    dbg,
    interrupts::hardware::InterruptIndex,
    malloc, mem_map,
    memory::handler::{map, map_frame},
    pci::PciDevice,
    time::mdelay,
};

use super::{driver::GenericDisk, DiskLoc};
impl GenericDisk for NVMeDisk {
    fn loc(&self) -> &super::DiskLoc {
        return &self.loc
    }
}
impl core::fmt::Display for NVMeDisk {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        return f.write_str(alloc::format!("NVME {:?}", self.loc).as_str())
    }
}
#[derive(Debug)]
pub struct NVMeDisk {
    loc: DiskLoc,
}
#[derive(Debug)]
pub enum NVMeControllerInitError {
    FatalError,
}

// Following https://nvmexpress.org/wp-content/uploads/NVM-Express-Base-Specification-2.0d-2024.01.11-Ratified.pdf P125
pub fn init(nvme_pci: &PciDevice) -> Result<Vec<&'static NVMeDisk>, NVMeControllerInitError> {
    let bar0 = nvme_pci.raw.determine_mem_base(0).unwrap();
    let bar0 = match bar0 {
        crate::pci::PciMemoryBase::MemorySpace(mem) => mem.as_u64(),
        crate::pci::PciMemoryBase::IOSpace(_) => todo!(),
    };
    // Enable bus mastering & memory space
    let mut command = nvme_pci.raw.command;
    command.set_bit(2, true);
    command.set_bit(1, true);
    command.set_bit(0, false);
    nvme_pci
        .raw
        .location
        .pci_write(crate::pci::PCI_COMMAND, command as u32);
    for i in 0..64 {
        mem_map!(
            frame_addr = bar0 + (0x1000 * i),
            PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::NO_CACHE
                | PageTableFlags::WRITE_THROUGH
        );
    }
    let controller = unsafe { NVMeRegisters::new(bar0 as usize) };
    log::info!(
        "[NVME] Found NVMe device with version {}.{}.{}, maximum queues supported: {}",
        controller.version >> 16,
        controller.version >> 8 & 0xff,
        controller.version & 0xff,
        controller.get_max_queue_entries()
    );

    // Following DGOS https://github.com/doug65536/dgos/blob/master/kernel/device/nvme/nvme.cc
    // Disable the controller
    controller.controller_config.set_enable(0);

    // let timeout_ms = controller.controller_caps.timeout() * 500;
    //TODO Make a decrement timer
    while controller.controller_status.ready() != 0 {}

    // dbg!(controller.controller_caps);

    // Attempt to use 64KB/16KB submission/completion queue sizes
    let mut queue_slots = 1024; // Max 4096
    let max_queue_slots = controller.controller_caps.mqes() + 1;

    if queue_slots > max_queue_slots {
        queue_slots = max_queue_slots
    }

    // Size of one queue, in bytes
    let mut queue_bytes = queue_slots * core::mem::size_of::<NVMeCommand>() as u64
        + queue_slots * core::mem::size_of::<NVMeCompletion>() as u64;

    let queue_count = 1;
    queue_bytes *= queue_count;

    let admin_submission_queue = Vec::<u8>::with_capacity(queue_bytes as usize);
    let admin_submission_queue_base_addr = admin_submission_queue.leak().as_ptr() as u64;
    // dbg!(queue_bytes); // 81920 bytes on QEMU
    // for page_addr in 0..queue_bytes.div_ceil(4096) {
    //     let page = Page::containing_address(VirtAddr::new(page_addr+admin_submission_queue_base_addr));
    //     map(page, PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE);
    // }

    // 7.6.1 3) The admin queue should be configured
    controller
        .admin_queue_attrs
        .set_admin_completion_queue_size(queue_slots.try_into().unwrap());
    controller
        .admin_queue_attrs
        .set_admin_submission_queue_size(queue_slots.try_into().unwrap());

    // Submission queue address
    controller.admin_submission_queue_base_addr = admin_submission_queue_base_addr;

    // 3.1.10 The vector for the admin queues is always 0
    // Completion queue address
    controller.admin_completion_queue_base_addr = admin_submission_queue_base_addr
        + (queue_count * queue_slots * core::mem::size_of::<NVMeCommand>() as u64);

    // 7.6.1 4) The controller settings should be configured
    let mut cc = NVMeControllerConfig(0);
    cc.set_io_completion_queue_entry_size(4); // 2^4 == 16 NVMeCompletion
    cc.set_io_submission_queue_entry_size(6); // 2^6 == 64
    cc.set_mps(0); // 4096 bytes for page size
    cc.set_io_commandset_selected(0);

    // Try to enable weighted round robin with urgent if capable
    if controller.controller_caps.ams() == 1 {
        cc.set_arbitration_mechanism_selected(1);
    }
    controller.controller_config = cc;
    // Set enable with a separate write
    controller.controller_config.set_enable(1);

    // 7.6.1 4) Wait for ready
    loop {
        if controller.controller_status.ready() != 0 {
            break;
        } else if controller.controller_status.fatal() != 0 {
            return Err(NVMeControllerInitError::FatalError);
        }
    };

    // Read the doorbell stride
    let doorbell_shift = controller.controller_caps.dstrd() + 1;

    // 7.4 Initialize queues

    // Initialize just the admin queue until we are sure about the
    // number of supported queues

    // let mut config = regs.controller_config;
    // regs.controller_config = config.set_bit(0, false).clone();
    // let mut i = 500;
    // while i > 0 {
    //     i -= 1;
    //     let status = regs.controller_status;
    //     let st = status.get_bit(0);
    //     if !st {
    //         break;
    //     }
    //     mdelay(10);
    // }
    // if i == 0 {
    //     log::warn!("NVMe device doesn't want to be disabled, skipping");
    //     return None;
    // }

    // // Checking page size following https://github.com/LemonOSProject/LemonOS/blob/master/Kernel/include/Storage/NVMe.h#L18
    // if 0x1000 << ((regs.controller_caps >> 52) & 0xf) < 4096
    //     || 0x1000 << ((regs.controller_caps >> 48) & 0xf) > 4096
    // {
    //     log::warn!("NVMe device doesn't support 4k memory page size, skipping");
    //     return None;
    // }
    // // We can now set page size to 4096
    // regs.controller_config =
    //     (regs.controller_config & !NVME_CFG_MPS(NVME_CFG_MPS_MASK as u32)) | NVME_CFG_MPS(0); // 2^12+0 = 4096

    // regs.set_command_set(NVME_CONFIG_CMDSET_NVM);

    // regs.controller_config |= NVME_CFG_DEFAULT_IOCQES | NVME_CFG_DEFAULT_IOSQES;

    // regs.admin_completion_queue_base_addr =
    //     crate::malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE)?.as_u64();
    // regs.admin_submission_queue_base_addr =
    //     crate::malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE)?.as_u64();

    // let mut adm_queue = NVMeQueue {
    //     queue_id: 0, /* admin queue ID is 0 */
    //     completion_base: VirtAddr::new(regs.admin_completion_queue_base_addr),
    //     submission_base: VirtAddr::new(regs.admin_submission_queue_base_addr),
    //     completion_db: regs.get_completion_doorbell(0),
    //     submission_db: regs.get_submission_doorbell(0),
    //     completion_queue_size: 4096.min(regs.get_max_queue_entries()),
    //     submission_queue_size: 4096.min(regs.get_max_queue_entries()),
    // };

    // regs.admin_queue_attrs = 0;

    // regs.set_admin_completion_queue_size(
    //     adm_queue.completion_queue_size / core::mem::size_of::<NVMeCompletion>() as u16,
    // );
    // regs.set_admin_submission_queue_size(
    //     adm_queue.submission_queue_size / core::mem::size_of::<NVMeCommand>() as u16,
    // );

    // log::info!(
    //     "[NVMe] CQ size: {}, SQ size: {}",
    //     (regs.admin_queue_attrs >> 16) + 1,
    //     (regs.admin_queue_attrs & 0xffff) + 1
    // );
    // regs.enable();
    // let mut i = 500;
    // while i > 0 {
    //     i -= 1;
    //     let status = regs.controller_status;
    //     let st = status.get_bit(0);
    //     if st {
    //         break;
    //     }
    //     mdelay(10);
    // }
    // if i == 0 {
    //     log::warn!("NVMe device doesn't want to be enabled, skipping");
    //     return None;
    // }
    // if (regs.controller_status & NVME_CSTS_FATAL as u32 != 0) {
    //     log::warn!("[NVMe] Controller fatal error! (NVME_CSTS_FATAL set)");
    //     return None;
    // }
    // dbg!(adm_queue);
    // adm_queue.identify();
    // // loop {
    // mdelay(100);
    // dbg!(regs);
    // let completion = unsafe {
    //     core::slice::from_raw_parts(
    //         regs.admin_completion_queue_base_addr as *const NVMeCompletion,
    //         4,
    //     )
    // };
    // dbg!(completion);

    // // }
    let mut disks = Vec::new();

    return Ok(disks)
}
fn bit_log2(n: u64) -> u64 {
    // if n == 0 {
    //     return 0; // Logarithm of 0 is undefined, returning 0 as a default
    // }

    let mut result = 0;
    let mut value = n;

    while value > 1 {
        value >>= 1;
        result += 1;
    }

    return result
}

#[derive(Debug)]
pub struct NVMeQueue {
    pub queue_id: u16,
    pub completion_base: VirtAddr,
    pub submission_base: VirtAddr,
    pub completion_db: u32,
    pub submission_db: u32,
    pub completion_queue_size: u16,
    pub submission_queue_size: u16,
}
impl NVMeQueue {
    // fn identify(&mut self) {
    //     let id = malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE).unwrap();
    //     // TODO Proper command creation
    //     let command = NVMeSubmission {
    //         opcode: 0x6,
    //         fuse_and_psdt: 0,
    //         command_id: 0,
    //         ns_id: 0,
    //         _reserved: 0,
    //         metadata_ptr: 0,
    //         prp1: 0,
    //         prp2: 0,
    //         command: NVMeCommand {
    //             raw: [1, 0, 0, 0, 0, 0],
    //             header: todo!(),
    //         },
    //     }; //TODOOOOOOOOOOOOOOOOOOOOOO
    //     let subs = unsafe {
    //         core::slice::from_raw_parts_mut(self.submission_base.as_u64() as *mut NVMeSubmission, 4)
    //     };
    //     subs[0] = command;
    //     dbg!((unsafe { *(self.submission_db as *mut u32) }));
    //     unsafe {
    //         *(self.submission_db as *mut u32) = 1;
    //     }
    // }
}
#[repr(C, packed)]
pub struct NVMeCommand {
    header: [u32; 10],
    raw: [u32; 6],
}

#[repr(C, packed)]
pub struct NVMeSubmission {
    opcode: u8,
    fuse_and_psdt: u8,
    command_id: u16,
    ns_id: u32,
    _reserved: u64,
    metadata_ptr: u64,
    prp1: u64,
    prp2: u64,
    command: NVMeCommand,
}
#[derive(Debug)]
#[repr(C, packed)]
pub struct NVMeCompletion {
    dw0: u32,        // DWORD 0, Command specific
    _reserved: u32,  // DWORD 1, Reserved
    sq_head: u16,    // DWORD 2, Submission queue (indicated in sqID) head pointer
    sq_id: u16,      // DWORD 2, Submission queue ID
    command_id: u16, // Id of the command being completed
    phase_tag_and_status: u16, // DWORD 3, Changed when a completion queue entry is new
                     // status: u16,     // DWORD 3, Status of command being completed
}

// let int = nvme_pci.raw.int_pin;
// dbg!(int, nvme_pci.raw.command);
// //TODO The host configures the Admin Queue by setting the Admin Queue Attributes (AQA), Admin
// // Submission Queue Base Address (ASQ), and Admin Completion Queue Base Address (ACQ) to
// // appropriate values;
// let caps = regs.controller_caps;
// dbg!(regs);
// // if caps.get_bit(37 + 7) {
// //     dbg!(config);
// // }
// // regs.nvm_subsystem_reset = 0x4E564D65; // Reset NVM

// dbg!(regs.capas_doorbell_stride());
// let addr = 4096 * 390;
// //TODO Does the NVMe needs it ?
// //TODO Get a frame allocator for low memory
// let buffer = unsafe { core::slice::from_raw_parts_mut(addr as *mut u8, 4096) }; // Rolled a dice, I swear it's random 🤣
// mem_map!(frame_addr = addr, WRITABLE);
// let identify = SubmissionEntry::new_identify(IdentifyType::Controller, None, buffer);
// //
// let submission_addr = regs.admin_submission_queue;
// mem_map!(frame_addr = submission_addr, WRITABLE);
// mem_map!(frame_addr = submission_addr + 0x1000, WRITABLE);
// regs.add_submission_entry(identify);
// loop {
//     let q = regs.completion_queue();
//     // if q.len()>0 {
//     //     dbg!(q);
//     //     break
//     // }
//     let buffer = unsafe { core::slice::from_raw_parts(addr as *const u8, 4096) };
//     // dbg!(q);
//     if !all_zeroes(buffer) {
//         break;
//     }
// }
// Some(Vec::new())

bitfield::bitfield! {
    pub struct NVMeControllerCaps(u64);
    impl Debug;
    /// This field indicates the maximum individual queue size that the controller supports. For NVMe over PCIe implementations, this value applies to the I/O Submission Queues and I/O Completion Queues that the host creates. For NVMe over Fabrics implementations, this value applies to only the I/O Submission Queues that the host creates. This is a'0's based value. The minimum value is 1h, indicating two entries
    mqes, _: 15, 0; // Maximum Queue Entries Supported
    /// This bit is set to '1' if the controller requires that I/O Submission Queues and I/O Completion Queues are required to be physically contiguous. This bit is cleared to '0' if the controller supports I/O Submission Queues and I/O Completion Queues that are not physically contiguous. If this bit is set to '1', then the Physically Contiguous bit (CDW11.PC) in the Create I/O Submission Queue and Create I/O Completion Queue commands shall be set to '1'. For I/O controllers and Discovery controllers using a message-based transport, this property shall be set to a value of 1h
    cqr, _: 16, 16; // Contiguous Queues Required
    /// This field is bit significant and indicates the optional arbitration mechanisms supported by the controller. If a bit is set to ‘1’, then the corresponding arbitration mechanism is supported by the controller. Refer to section 3.4.4 for arbitration details. Bits Definition 1 Vendor Specific 0 Weighted Round Robin with Urgent Priority Class The round robin arbitration mechanism is not listed since all controllers shall support this arbitration mechanism. For Discovery controllers, this property shall be cleared to 0h
    ams, _: 18, 17; // Arbitration Mechanism Supported
    /// This is the worst-case time that host software should wait for CSTS.RDY to transition from: a) ‘0’ to ‘1’ after CC.EN transitions from ‘0’ to ‘1’; or b) ‘1’ to ‘0’ after CC.EN transitions from ‘1’ to ‘0’. This worst-case time may be experienced after events such as an abrupt shutdown or activation of a new firmware image; typical times are expected to be much shorter. This field is in 500 millisecond units. The maximum value of this field is FFh, which indicates a 127.5 second timeout. If the Controller Ready Independent of Media Enable (CC.CRIME) bit is cleared to ‘0’ and the worst-case time for CSTS.RDY to change state is due to enabling the controller after CC.EN transitions from ‘0’ to ‘1’, then this field shall be set to: a) the value in Controller Ready With Media Timeout (CRTO.CRWMT); or b) FFh if CRTO.CRWMT is greater than FFh. If the Controller Ready Independent of Media Enable (CC.CRIME) bit is set to ‘1’ and the worst-case time for CSTS.RDY to change state is due to enabling the controller after CC.EN transitions from ‘0’ to ‘1’, then this field shall be set to: a) the value in Controller Ready Independent of Media Timeout (CRTO.CRIMT); or b) FFh if CRTO.CRIMT is greater than FFh. Controllers that support the CRTO register (refer to Figure 62) are able to indicate larger timeouts for enabling the controller. Host software should use the value in CRTO.CRWMT or CRTO.CRIMT depending on the controller ready mode indicated by CC.CRIME to determine the worst-case timeout for CSTS.RDY to transition from ‘0’ to ‘1’ after CC.EN transitions from ‘0’ to ‘1’. Host software that is based on revisions earlier than NVM Express Base Specification, Revision 2.0 is not required to wait for more than 127.5 seconds for CSTS.RDY to transition. Refer to sections 3.5.3 and 3.5.4 for more information.
    timeout, _: 31, 24; // TO
    /// Each Submission Queue and Completion Queue Doorbell property is 32-bits in size. This property indicates the stride between doorbell properties. The stride is specified as (2 ^ (2 + DSTRD)) in bytes. A value of 0h indicates a stride of 4 bytes, where the doorbell properties are packed without reserved space between each property. Refer to section 8.8. For NVMe over Fabrics I/O controllers, this property shall be cleared to a fixed value of 0h.
    dstrd, _: 35, 32; // Doorbell Stride
    /// This field indicates the minimum host memory page size that the controller supports. The minimum memory page size is (2 ^ (12 + MPSMIN)). The host shall not configure a memory page size in CC.MPS that is smaller than this value. For Discovery controllers this shall be cleared to 0h
    mpsmin, _: 51, 48; // Memory Page Size Minimum
    /// This field indicates the maximum host memory page size that the controller supports. The maximum memory page size is (2 ^ (12 + MPSMAX)). The host shall not configure a memory page size in CC.MPS that is larger than this value. For Discovery controllers this field shall be cleared to 0h
    mpsmax, _: 55, 52; // Memory Page Size Maximum
}
bitfield::bitfield! {
    /// Host software shall set the Arbitration Mechanism Selected, the Memory Page Size (mps), and the I/O Command Set Selected (CC.CSS) to valid values
    /// prior to enabling the controller by setting CC.EN to '1'. Attempting to create an I/O queue before initializing
    /// the I/O Completion Queue Entry Size (CC.IOCQES) and the I/O Submission Queue Entry Size
    /// (CC.IOSQES) should cause a controller to abort a Create I/O Completion Queue command or a Create I/O
    /// Submission Queue command with a status code of Invalid Queue Size
    pub struct NVMeControllerConfig(u32);
    impl Debug;
    /// When set to '1', then the controller shall process commands. When cleared to '0', then the controller shall not process commands nor post completion queue entries to Completion Queues. When the host modifies CC to clear this bit from '1' to '0', the controller is reset (i.e., a Controller Reset, refer to section 3.7.2). That reset deletes all I/O Submission Queues and I/O Completion Queues, resets the Admin Submission Queue and Completion Queue, and brings the hardware to an idle state. That reset does not affect transport specific state (e.g. PCI Express registers including MMIO MSI-X registers), nor the Admin Queue properties (AQA, ASQ, or ACQ). All other controller properties defined in this section and internal controller state (e.g., Feature values defined in section 5.27.1 that are not persistent across power states) are reset to their default values. The controller shall ensure that there is no impact (e.g., data loss) caused by that Controller Reset to the results of commands that have had corresponding completion queue entries posted to an I/O Completion Queue prior to that Controller Reset. Refer to section 3.6. When this bit is cleared to '0', the CSTS.RDY bit is cleared to '0' by the controller once the controller is ready to be re-enabled. When this bit is set to '1', the controller sets CSTS.RDY to '1' when it is ready to process commands. CSTS.RDY may be set to '1' before namespace(s) are ready to be accessed. Setting this bit from a '0' to a '1' when CSTS.RDY is a '1' or clearing this bit from a '1' to a '0' when CSTS.RDY is cleared to '0' has undefined results. The Admin Queue properties (AQA, ASQ, and ACQ) are only allowed to be modified when this bit is cleared to '0'. If an NVM Subsystem Shutdown is in progress or is completed (i.e., CSTS.ST is set to '1', and CSTS.SHST is set to 01b or 10b), then writes to this field modify the field value but have no effect. Refer to section 3.6.3 for details
    enable, set_enable: 0, 0;
    /// This field specifies the I/O Command Set or Sets that are selected. This field shall only be changed when the controller is disabled (i.e., CC.EN is cleared to ‘0’). The I/O Command Set or Sets that are selected shall be used for all I/O Submission Queues.
    ///Value Definition
    /// Table in pdf =)
    /// For Discovery controllers, this property shall be cleared to 000b.
    io_commandset_selected, set_io_commandset_selected: 6, 4; // CSS
    /// This field indicates the host memory page size. The memory page size is (2 ^ (12 + MPS)). Thus, the minimum host memory page size is 4 KiB and the maximum host memory page size is 128 MiB. The value set by host software shall be a supported value as indicated by the CAP.MPSMAX and CAP.MPSMIN fields. This field describes the value used for PRP entry size. This field shall only be modified when CC.EN is cleared to ‘0’. For Discovery controllers this property shall be cleared to 0h
    mps, set_mps: 10, 7; // Memory Page Size
    /// This field selects the arbitration mechanism to be used. This value shall only be changed when CC.EN is cleared to ‘0’. Host software shall only set this field to supported arbitration mechanisms indicated in CAP.AMS. If this field is set to an unsupported value, the behavior is undefined.
    /// For Discovery controllers, this value shall be cleared to 0h.
    /// Value Definition
    /// 000b Round Robin
    /// 001b Weighted Round Robin with Urgent Priority Class
    /// 010b to 110b Reserved
    /// 111b Vendor Specific
    arbitration_mechanism_selected, set_arbitration_mechanism_selected: 13, 11; // AMS
    /// This field defines the I/O submission queue entry size that is used for the selected I/O Command Set(s). The required and maximum values for this field are specified in the SQES field in the Identify Controller data structure in Figure 276 for each I/O Command Set. The value is in bytes and is specified as a power of two (2^n). If any I/O Submission Queues exist, then write operations that change the value in this field produce undefined results. If the controller does not support I/O queues, then this field shall be read-only with a value of 0h. For Discovery controllers, this field is reserved
    io_submission_queue_entry_size, set_io_submission_queue_entry_size: 19, 16; // I/O Submission Queue Entry Size (IOSQES)
    /// This field defines the I/O completion queue entry size that is used for the selected I/O Command Set(s). The required and maximum values for this field are specified in the CQES field in the Identify Controller data structure in Figure 276 for each I/O Command Set. The value is in bytes and is specified as a power of two (2^n). If any I/O Completion Queues exist, then write operations that change the value in this field produce undefined results. If the controller does not support I/O queues, then this field shall be read-only with a value of 0h. For Discovery controllers, this field is reserved.
    io_completion_queue_entry_size, set_io_completion_queue_entry_size: 23, 20; // I/O completion Queue Entry Size (IOSCQES)
}
bitfield::bitfield! {
    pub struct NVMeControllerStatus(u32);
    impl Debug;
    /// : This bit is set to ‘1’ when the controller is ready to process submission queue entries after CC.EN is set to ‘1’. This bit shall be cleared to ‘0’ when CC.EN is cleared to ‘0’ once the controller is ready to be re-enabled. Commands should not be submitted to the controller until this bit is set to ‘1’ after the CC.EN bit is set to ‘1’. Failure to follow this recommendation produces undefined results. Refer to the definition of CAP.TO, sections 3.5.3, and 3.5.4 for timing information related to this field. If an NVM Subsystem Shutdown has completed that affects this controller (i.e., CSTS.ST is set to ‘1’ and CSTS.SHST is set to 10b), then an NVM Subsystem Reset is required before this bit is allowed to be set to ‘1’. Refer to section 3.6.3.
    ready, _: 0, 0;
    /// This bit is set to ’1’ when a fatal controller error occurred that could not be communicated in the appropriate Completion Queue. This bit is cleared to ‘0’ when a fatal controller error has not occurred. Refer to section 9.5. The reset value of this bit is set to '1' when a fatal controller error is detected during controller initialization.
    fatal, _: 1, 1; // Controller Fatal Status (CFS)
}
impl NVMeControllerStatus {}
bitfield::bitfield! {
    pub struct NVMeControllerAdminQueueAttributes(u32);
    impl Debug;
    /// Defines the size of the Admin Completion Queue in entries. Refer to section 3.3.3.2.2. Enabling a controller while this field is cleared to 0h produces undefined results. The minimum size of the Admin Completion Queue is two entries. The maximum size of the Admin Completion Queue is 4,096 entries. This is a 0’s based value
    admin_completion_queue_size, set_admin_completion_queue_size: 27, 16; // ACQS
    ///  Defines the size of the Admin Submission Queue in entries. Refer to section 3.3.3.2.2. Enabling a controller while this field is cleared to 0h produces undefined results. The minimum size of the Admin Submission Queue is two entries. The maximum size of the Admin Submission Queue is 4,096 entries. This is a 0’s based value.
    admin_submission_queue_size, set_admin_submission_queue_size: 11, 0; // ASQS

}
struct NVMeControllerSubsystemReset(u32);
impl NVMeControllerSubsystemReset {
    pub fn reset(&mut self) {
        self.0 = 0x4E564D65;
    }
}
#[repr(C)]
struct NVMeRegisters {
    /// CAP
    controller_caps: NVMeControllerCaps,
    version: u32,
    /// INTMS
    interrupt_mask_set: u32,
    /// INTMC
    interrupt_mask_clear: u32,
    /// CC
    controller_config: NVMeControllerConfig,
    /// Something but idk, following the bad doc wiki.osdev.org/NVMe
    _pad: u32,
    /// CSTS
    controller_status: NVMeControllerStatus,
    /// NSSR
    nvm_subsystem_reset: u32,
    /// AQA
    admin_queue_attrs: NVMeControllerAdminQueueAttributes,
    /// ASQ
    admin_submission_queue_base_addr: u64,
    /// ACQ
    admin_completion_queue_base_addr: u64,
    /// CMBLOC
    controller_mem_buffer_location: u32,
    /// CMBSZ - Controller Memory Buffer Size.
    controller_mem_buffer_size: u32,
    /// BPINFO - Boot Partition Information
    boot_partition_info: u32,
    /// BPRSEL - Boot Partition Read Select . ..
    boot_partition_read_select: u32,
    // boot_partition_memory_buffer_location: u16,
    // controller_memory_buffer_memory_space_control: u32,
    // ...
}
impl NVMeRegisters {
    // fn set_admin_completion_queue_size(&mut self, sz: u16) {
    //     self.admin_queue_attrs = (self.admin_queue_attrs
    //         & !NVME_AQA_ACQS(NVME_AQA_AQS_MASK) as u32)
    //         | NVME_AQA_ACQS(sz as u64 - 1) as u32; // 0's based
    // }

    // fn set_admin_submission_queue_size(&mut self, sz: u16) {
    //     self.admin_queue_attrs = (self.admin_queue_attrs
    //         & !NVME_AQA_ASQS(NVME_AQA_AQS_MASK) as u32)
    //         | NVME_AQA_ASQS(sz as u64 - 1) as u32; // 0's based
    // }

    fn get_max_queue_entries(&self) -> u16 {
        let max = (self.controller_caps.0 & 0xffff) as u16;
        if max == 0 {
            return u16::MAX
        } else {
            return max + 1
        }
    }
    // fn set_command_set(&mut self, set: u8) {
    //     self.controller_config = (self.controller_config & !NVME_CFG_CSS(NVME_CFG_CSS_MASK) as u32)
    //         | NVME_CFG_CSS(set) as u32;
    // }
    // fn get_completion_doorbell(&mut self, queue_id: u16) -> u32 {
    //     return (addr_of!(*self) as u32)
    //         + 0x1000
    //         + (2 * queue_id as u32 + 1) * (4 << self.capas_doorbell_stride());
    // }
    // fn get_submission_doorbell(&mut self, queue_id: u16) -> u32 {
    //     return (addr_of!(*self) as u32)
    //         + 0x1000
    //         + (2 * queue_id as u32) * (4 << self.capas_doorbell_stride());
    // }
    // fn enable(&mut self) {
    //     self.controller_config |= NVME_CFG_ENABLE;
    // }

    /// # Safety
    /// Ensure that bar0 address is the proper base for the NVMe registers
    pub unsafe fn new(bar0: usize) -> &'static mut Self {
        unsafe { return &mut *(bar0 as *mut Self) }
    }
    ///This bit is set to '1' when the controller is ready to process submission
    ///queue entries after CC.EN is set to '1'. This bit shall be cleared to '0' when CC.EN is
    ///cleared to '0' once the controller is ready to be re-enabled. Commands should not be
    ///submitted to the controller until this bit is set to '1' after the CC.EN bit is set to '1'.
    ///Failure to follow this recommendation produces undefined results. Refer to the
    ///definition of CAP.TO, sections 3.5.3, and 3.5.4 for timing information related to this
    ///field.
    // pub fn ready(&self) -> bool {
    //     let status = self.controller_status;
    //     status.get_bit(0)
    // }
    // fn db_stride(&self) -> u64 {
    //     1 << (((self.controller_caps) >> 32) & 0xf)
    // }
    pub fn base(&self) -> usize {
        return core::ptr::addr_of!(self.controller_caps) as usize
    }
    // pub fn submission_queue(&self) -> Vec<SubmissionEntry> {
    //     // Can we know the size of the vec ? If so with_capacity()
    //     let mut queue = Vec::new();
    //     for i in 0..1000u64 {
    //         // Max queues is 64Kib
    //         let addr = ((self.base() as u64 + 0x1000) + (2 * i) * self.db_stride());
    //         let v = unsafe { &*(addr as *const SubmissionEntry) }.clone();
    //         if all_zeroes(any_as_u8_slice(&v)) {
    //             break;
    //         }
    //         queue.push(v);
    //     }
    //     queue
    // }
    ///((self.base() as u64+0x1000)+(2*0+1)*self.db_stride())
    pub fn add_submission_entry(&mut self, entry: SubmissionEntry) {
        unsafe { *(self.admin_submission_queue_base_addr as *mut SubmissionEntry) = entry }
    }
    pub fn completion_queue(&self) -> Vec<CompletionEntry> {
        // Can we know the size of the vec ? If so with_capacity()
        let mut queue = Vec::new();
        for i in 0..1000_u64 {
            // Max queues is 64Kib
            let v = unsafe { &*(self.admin_completion_queue_base_addr as *const CompletionEntry) }
                .clone();
            if all_zeroes(any_as_u8_slice(&v)) {
                break;
            }
            queue.push(v);
        }
        return queue
    }
    // CAP.DSTRD
    // pub fn capas_doorbell_stride(&self) -> u8 {
    //     let capas = self.controller_caps;
    //     capas.get_bits(32..35).try_into().unwrap()
    // }
    // /// CAP.CSS
    // pub fn capas_command_sets_supported(&self) -> u8 {
    //     let capas = self.controller_caps;
    //     capas.get_bits(37..44).try_into().unwrap()
    // }
}
#[derive(Clone)]
#[repr(C, packed)]
struct SubmissionEntry {
    command: CommandDword0,
    namespace_id: u32,
    reserved: [u32; 2],
    metadata_ptr: u64,
    /// 2 PRPs see https://wiki.osdev.org/NVMe#PRP
    data_ptr: [u64; 2],
    command_specific: [u32; 6],
}
impl SubmissionEntry {
    pub fn new_identify(
        to_identify: IdentifyType,
        namespace_id: Option<u32>,
        buffer: &mut [u8],
    ) -> Self {
        let addr = addr_of!(*buffer) as *const () as u64;
        dbg!(addr_of!(*buffer));
        let (namespace_id, first_command) = match to_identify {
            IdentifyType::Namespace => (namespace_id.unwrap(), 0),
            IdentifyType::Controller => (0, 1),
            IdentifyType::NamespaceList => (0, 2),
        };
        return Self {
            command: CommandDword0::new(0x6, 0, 0, 1),
            namespace_id,
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: [addr, 0],
            command_specific: [first_command, 0, 0, 0, 0, 0],
        }
    }
}
impl core::fmt::Debug for SubmissionEntry {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let command = self.command;
        let namespace_id = self.namespace_id;
        let reserved = self.reserved;
        let metadata_ptr = self.metadata_ptr;
        let data_ptr = self.data_ptr;
        let command_specific = self.command_specific;
        return f.debug_struct("SubmissionEntry")
            .field("command", &command)
            .field("namespace_id", &namespace_id)
            .field("reserved", &reserved)
            .field("metadata_ptr", &metadata_ptr)
            .field("data_ptr", &data_ptr)
            .field("command_specific", &command_specific)
            .finish()
    }
}
#[derive(Debug, Clone, Copy)]
struct CommandDword0 {
    pub opcode: u8,
    raw: u8,
    /// This is put in the completion queue entry
    pub command_id: u16,
}
impl CommandDword0 {
    pub fn new(opcode: u8, fused_op: u8, prp_or_sql_select: u8, command_id: u16) -> Self {
        return Self {
            opcode,
            command_id,
            raw: *fused_op
                .get_bits(0..2)
                .set_bits(6..8, prp_or_sql_select.get_bits(6..8)),
        }
    }
    /// 0 indicates normal operation
    /// This is 2 bits
    pub fn fused_operation(self) -> u8 {
        return self.raw.get_bits(0..2)
    }
    /// 0 indicates PRPs.
    /// This is 2 bits
    pub fn prp_or_sgl_selection(self) -> u8 {
        return self.raw.get_bits(6..8)
    }
}

#[derive(Debug, Clone)]
struct CompletionEntry {
    pub command_specific: u32,
    _reserved: u32,
    pub submission_queue_head_ptr: u16,
    pub submission_queue_id: u16,
    pub command_id: u16,
    _status: u16,
}
impl CompletionEntry {
    /// Toggled when entry written
    pub fn phase_bit(&self) -> bool {
        return self._status.get_bit(0)
    }
    /// 0 on success
    /// 14bits
    pub fn status(&self) -> u16 {
        return self._status.get_bits(1..)
    }
}

enum Commands {
    Admin(AdminCommand),
    IO(IOCommand),
}
enum AdminCommand {
    CreateIOSubmissionQueue,
    CreateIOCompletionQueue,
    Identify,
}
enum IOCommand {
    Read,
    Write,
}

enum IdentifyType {
    Namespace = 0,
    Controller = 1,
    NamespaceList = 2,
}

const NVME_CAP_CMBS: u64 = (1 << 57); // Controller memory buffer supported
const NVME_CAP_PMRS: u64 = (1 << 56); // Persistent memory region supported
const NVME_CAP_BPS: u64 = (1 << 45); // Boot partition support
const NVME_CAP_NVM_CMD_SET: u64 = (1 << 37); // NVM command set supported
const NVME_CAP_NSSRS: u64 = (1 << 36); // NVM subsystem reset supported
const NVME_CAP_CQR: u32 = (1 << 16); // Contiguous Queues Required

const NVME_CAP_MPS_MASK: u8 = 0xf;
const NVME_CAP_MPSMAX: fn(u64) -> u64 = |x| return ((x >> 52) & NVME_CAP_MPS_MASK as u64); // Max supported memory page size (2 ^ (12 + MPSMAX))
const NVME_CAP_MPSMIN: fn(u64) -> u64 = |x| return ((x >> 48) & NVME_CAP_MPS_MASK as u64); // Min supported memory page size (2 ^ (12 + MPSMIN))

const NVME_CAP_DSTRD_MASK: u64 = 0xf;
const NVME_CAP_DSTRD: fn(u64) -> u64 = |x| return (((x) >> 32) & NVME_CAP_DSTRD_MASK); // Doorbell stride (2 ^ (2 + DSTRD)) bytes

const NVME_CAP_MQES_MASK: u64 = 0xffff;
const NVME_CAP_MQES: fn(u64) -> u64 = |x| return ((x) & NVME_CAP_MQES_MASK); // Maximum queue entries supported

const NVME_CFG_MPS_MASK: u8 = 0xf;
const NVME_CFG_MPS: fn(u32) -> u32 = |x| return (((x) & NVME_CFG_MPS_MASK as u32) << 7); // Host memory page size (2 ^ (12 + MPSMIN))
const NVME_CFG_CSS_MASK: u8 = 0b111; // Command set selected
const NVME_CFG_CSS: fn(u8) -> u8 = |x| return (((x) & NVME_CFG_CSS_MASK) << 4);
const NVME_CFG_ENABLE: u32 = (1 << 0);
const NVME_CONFIG_CMDSET_NVM: u8 = 0;
const NVME_CFG_DEFAULT_IOCQES: u32 = (4 << 20); // 16 bytes so log2(16) = 4
const NVME_CFG_DEFAULT_IOSQES: u32 = (6 << 16); // 64 bytes so log2(64) = 6

const NVME_CSTS_FATAL: u8 = (1 << 1);
const NVME_CSTS_READY: u8 = (1 << 0); // Set to 1 when the controller is ready to accept submission queue doorbell writes
const NVME_CSTS_NSSRO: u8 = (1 << 4); // NVM Subsystem reset occurred

const NVME_AQA_AQS_MASK: u64 = 0xfff; // Admin queue size mask
const NVME_AQA_ACQS: fn(u64) -> u64 = |x| return (((x) & NVME_AQA_AQS_MASK) << 16); // Admin completion queue size
const NVME_AQA_ASQS: fn(u64) -> u64 = |x| return ((x) & NVME_AQA_AQS_MASK); // Admin submission queue size

// "NVME", initiates a reset
const NVME_NSSR_RESET_VALUE: u64 = 0x4E564D65;
