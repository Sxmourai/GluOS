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
    memory::handler::map_frame,
    pci::PciDevice,
    time::mdelay,
};

use super::{driver::GenericDisk, DiskLoc};
impl GenericDisk for NVMeDisk {
    fn loc(&self) -> &super::DiskLoc {
        &self.loc
    }
}
impl core::fmt::Display for NVMeDisk {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(alloc::format!("NVME {:?}", self.loc).as_str())
    }
}
#[derive(Debug)]
pub struct NVMeDisk {
    loc: DiskLoc,
}
pub fn init(nvme_pci: &PciDevice) -> Option<Vec<&'static NVMeDisk>> {
    if true {
        return None;
    }
    log::debug!("{}", nvme_pci);
    let bar0 = nvme_pci.raw.determine_mem_base(0).unwrap().as_u64();
    // Enable bus mastering & memory space
    let mut command = nvme_pci.raw.command;
    command.set_bit(2, true);
    command.set_bit(1, true);
    command.set_bit(0, true);
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
    let regs = unsafe { NVMeRegisters::new(bar0 as usize) };
    log::info!(
        "Found NVMe device with version {}.{}.{}, maximum queues supported: {}",
        regs.version >> 16,
        regs.version >> 8 & 0xff,
        regs.version & 0xff,
        regs.get_max_queue_entries()
    );

    // Disable
    let mut config = regs.controller_config;
    regs.controller_config = *config.set_bit(0, false);
    let c = regs.controller_config;
    let mut i = 500;
    while i > 0 {
        i -= 1;
        let status = regs.controller_status;
        let st = status.get_bit(0);
        if !st {
            break;
        }
        mdelay(10);
    }
    if i == 0 {
        log::warn!("NVMe device doesn't want to be disabled, skipping");
        return None;
    }

    // Checking page size following https://github.com/LemonOSProject/LemonOS/blob/master/Kernel/include/Storage/NVMe.h#L18
    if 0x1000 << ((regs.controller_caps >> 52) & 0xf) < 4096
        || 0x1000 << ((regs.controller_caps >> 48) & 0xf) > 4096
    {
        log::warn!("NVMe device doesn't support 4k memory page size, skipping");
        return None;
    }
    // We can now set page size to 4096
    regs.controller_config =
        (regs.controller_config & !NVME_CFG_MPS(NVME_CFG_MPS_MASK as u32)) | NVME_CFG_MPS(0); // 2^12+0 = 4096

    regs.set_command_set(NVME_CONFIG_CMDSET_NVM);

    regs.controller_config |= NVME_CFG_DEFAULT_IOCQES | NVME_CFG_DEFAULT_IOSQES;

    regs.admin_completion_queue =
        crate::malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE)?.as_u64();
    regs.admin_submission_queue =
        crate::malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE)?.as_u64();

    let mut adm_queue = NVMeQueue {
        queue_id: 0, /* admin queue ID is 0 */
        completion_base: VirtAddr::new(regs.admin_completion_queue),
        submission_base: VirtAddr::new(regs.admin_submission_queue),
        completion_db: regs.get_completion_doorbell(0),
        submission_db: regs.get_submission_doorbell(0),
        completion_queue_size: 4096.min(regs.get_max_queue_entries()),
        submission_queue_size: 4096.min(regs.get_max_queue_entries()),
    };

    regs.admin_queue_attrs = 0;

    regs.set_admin_completion_queue_size(
        adm_queue.completion_queue_size / core::mem::size_of::<NVMeCompletion>() as u16,
    );
    regs.set_admin_submission_queue_size(
        adm_queue.submission_queue_size / core::mem::size_of::<NVMeCommand>() as u16,
    );

    log::info!(
        "[NVMe] CQ size: {}, SQ size: {}",
        (regs.admin_queue_attrs >> 16) + 1,
        (regs.admin_queue_attrs & 0xffff) + 1
    );
    regs.enable();
    let mut i = 500;
    while i > 0 {
        i -= 1;
        let status = regs.controller_status;
        let st = status.get_bit(0);
        if st {
            break;
        }
        mdelay(10);
    }
    if i == 0 {
        log::warn!("NVMe device doesn't want to be enabled, skipping");
        return None;
    }
    if (regs.controller_status & NVME_CSTS_FATAL as u32 != 0) {
        log::warn!("[NVMe] Controller fatal error! (NVME_CSTS_FATAL set)");
        return None;
    }
    dbg!(adm_queue);
    adm_queue.identify();
    // loop {
    mdelay(100);
    dbg!(regs);
    let completion = unsafe {
        core::slice::from_raw_parts(regs.admin_completion_queue as *const NVMeCompletion, 4)
    };
    dbg!(completion);

    // }

    return None;
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
    fn identify(&mut self) {
        let id = malloc!(PageTableFlags::PRESENT | PageTableFlags::WRITABLE).unwrap();
        // TODO Proper command creation
        let command = NVMeSubmission {
            opcode: 0x6,
            fuse_and_psdt: 0,
            command_id: 0,
            ns_id: 0,
            _reserved: 0,
            metadata_ptr: 0,
            prp1: 0,
            prp2: 0,
            command: NVMeCommand {
                raw: [1, 0, 0, 0, 0, 0],
            },
        }; //TODOOOOOOOOOOOOOOOOOOOOOO
        let subs = unsafe {
            core::slice::from_raw_parts_mut(self.submission_base.as_u64() as *mut NVMeSubmission, 4)
        };
        subs[0] = command;
        dbg!((unsafe { *(self.submission_db as *mut u32) }));
        unsafe {
            *(self.submission_db as *mut u32) = 1;
        }
    }
}
#[repr(C, packed)]
pub struct NVMeCommand {
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
#[derive(Debug)]
#[repr(C, packed)]
struct NVMeRegisters {
    controller_caps: u64,
    version: u32,
    interrupt_mask_set: u32,
    interrupt_mask_clear: u32,
    controller_config: u32,
    /// Something but idk, following the bad doc wiki.osdev.org/NVMe
    _pad: u32,
    controller_status: u32,
    nvm_subsystem_reset: u32,
    admin_queue_attrs: u32,
    admin_submission_queue: u64,
    admin_completion_queue: u64,
    controller_mem_buffer_location: u32,
    controller_mem_buffer_size: u32,
    boot_partition_info: u32,
    boot_partition_read_select: u32,
    // boot_partition_memory_buffer_location: u16,
    // controller_memory_buffer_memory_space_control: u32,
}
impl NVMeRegisters {
    fn set_admin_completion_queue_size(&mut self, sz: u16) {
        self.admin_queue_attrs = (self.admin_queue_attrs
            & !NVME_AQA_ACQS(NVME_AQA_AQS_MASK) as u32)
            | NVME_AQA_ACQS(sz as u64 - 1) as u32; // 0's based
    }

    fn set_admin_submission_queue_size(&mut self, sz: u16) {
        self.admin_queue_attrs = (self.admin_queue_attrs
            & !NVME_AQA_ASQS(NVME_AQA_AQS_MASK) as u32)
            | NVME_AQA_ASQS(sz as u64 - 1) as u32; // 0's based
    }

    fn get_max_queue_entries(&self) -> u16 {
        let max = (self.controller_caps & 0xffff) as u16;
        if (max <= 0) {
            return u16::MAX;
        } else {
            return max + 1;
        }
    }
    fn set_command_set(&mut self, set: u8) {
        self.controller_config = (self.controller_config & !NVME_CFG_CSS(NVME_CFG_CSS_MASK) as u32)
            | NVME_CFG_CSS(set) as u32;
    }
    fn get_completion_doorbell(&mut self, queue_id: u16) -> u32 {
        return (addr_of!(*self) as u32)
            + 0x1000
            + (2 * queue_id as u32 + 1) * (4 << self.capas_doorbell_stride());
    }
    fn get_submission_doorbell(&mut self, queue_id: u16) -> u32 {
        return (addr_of!(*self) as u32)
            + 0x1000
            + (2 * queue_id as u32) * (4 << self.capas_doorbell_stride());
    }
    fn enable(&mut self) {
        self.controller_config |= NVME_CFG_ENABLE;
    }

    /// # Safety
    /// Ensure that bar0 address is the proper base for the NVMe registers
    pub unsafe fn new(bar0: usize) -> &'static mut Self {
        unsafe { &mut *(bar0 as *mut Self) }
    }
    ///This bit is set to '1' when the controller is ready to process submission
    ///queue entries after CC.EN is set to '1'. This bit shall be cleared to '0' when CC.EN is
    ///cleared to '0' once the controller is ready to be re-enabled. Commands should not be
    ///submitted to the controller until this bit is set to '1' after the CC.EN bit is set to '1'.
    ///Failure to follow this recommendation produces undefined results. Refer to the
    ///definition of CAP.TO, sections 3.5.3, and 3.5.4 for timing information related to this
    ///field.
    pub fn ready(&self) -> bool {
        let status = self.controller_status;
        status.get_bit(0)
    }
    pub fn admin_completion_queue_size(&self) -> u16 {
        let attrs = self.admin_queue_attrs;
        attrs.get_bits(16..27).try_into().unwrap()
    }
    pub fn admin_submission_queue_size(&self) -> u16 {
        let attrs = self.admin_queue_attrs;
        attrs.get_bits(0..11).try_into().unwrap()
    }
    fn db_stride(&self) -> u64 {
        1 << (((self.controller_caps) >> 32) & 0xf)
    }
    pub fn base(&self) -> usize {
        core::ptr::addr_of!(self.controller_caps) as usize
    }
    pub fn submission_queue(&self) -> Vec<SubmissionEntry> {
        // Can we know the size of the vec ? If so with_capacity()
        let mut queue = Vec::new();
        for i in 0..1000u64 {
            // Max queues is 64Kib
            let addr = ((self.base() as u64 + 0x1000) + (2 * i) * self.db_stride());
            let v = unsafe { &*(addr as *const SubmissionEntry) }.clone();
            if all_zeroes(any_as_u8_slice(&v)) {
                break;
            }
            queue.push(v);
        }
        queue
    }
    ///((self.base() as u64+0x1000)+(2*0+1)*self.db_stride())
    pub fn add_submission_entry(&mut self, entry: SubmissionEntry) {
        unsafe { *(self.admin_submission_queue as *mut SubmissionEntry) = entry }
    }
    pub fn completion_queue(&self) -> Vec<CompletionEntry> {
        // Can we know the size of the vec ? If so with_capacity()
        let mut queue = Vec::new();
        for i in 0..1000u64 {
            // Max queues is 64Kib
            let v = unsafe { &*(self.admin_completion_queue as *const CompletionEntry) }.clone();
            if all_zeroes(any_as_u8_slice(&v)) {
                break;
            }
            queue.push(v);
        }
        queue
    }
    /// CAP.DSTRD
    pub fn capas_doorbell_stride(&self) -> u8 {
        let capas = self.controller_caps;
        capas.get_bits(32..35).try_into().unwrap()
    }
    /// CAP.CSS
    pub fn capas_command_sets_supported(&self) -> u8 {
        let capas = self.controller_caps;
        capas.get_bits(37..44).try_into().unwrap()
    }
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
        Self {
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
        f.debug_struct("SubmissionEntry")
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
        Self {
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
        self.raw.get_bits(0..2)
    }
    /// 0 indicates PRPs.
    /// This is 2 bits
    pub fn prp_or_sgl_selection(self) -> u8 {
        self.raw.get_bits(6..8)
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
        self._status.get_bit(0)
    }
    /// 0 on success
    /// 14bits
    pub fn status(&self) -> u16 {
        self._status.get_bits(1..)
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
const NVME_CAP_MPSMAX: fn(u64) -> u64 = |x| ((x >> 52) & NVME_CAP_MPS_MASK as u64); // Max supported memory page size (2 ^ (12 + MPSMAX))
const NVME_CAP_MPSMIN: fn(u64) -> u64 = |x| ((x >> 48) & NVME_CAP_MPS_MASK as u64); // Min supported memory page size (2 ^ (12 + MPSMIN))

const NVME_CAP_DSTRD_MASK: u64 = 0xf;
const NVME_CAP_DSTRD: fn(u64) -> u64 = |x| (((x) >> 32) & NVME_CAP_DSTRD_MASK); // Doorbell stride (2 ^ (2 + DSTRD)) bytes

const NVME_CAP_MQES_MASK: u64 = 0xffff;
const NVME_CAP_MQES: fn(u64) -> u64 = |x| ((x) & NVME_CAP_MQES_MASK); // Maximum queue entries supported

const NVME_CFG_MPS_MASK: u8 = 0xf;
const NVME_CFG_MPS: fn(u32) -> u32 = |x| (((x) & NVME_CFG_MPS_MASK as u32) << 7); // Host memory page size (2 ^ (12 + MPSMIN))
const NVME_CFG_CSS_MASK: u8 = 0b111; // Command set selected
const NVME_CFG_CSS: fn(u8) -> u8 = |x| (((x) & NVME_CFG_CSS_MASK) << 4);
const NVME_CFG_ENABLE: u32 = (1 << 0);
const NVME_CONFIG_CMDSET_NVM: u8 = 0;
const NVME_CFG_DEFAULT_IOCQES: u32 = (4 << 20); // 16 bytes so log2(16) = 4
const NVME_CFG_DEFAULT_IOSQES: u32 = (6 << 16); // 64 bytes so log2(64) = 6

const NVME_CSTS_FATAL: u8 = (1 << 1);
const NVME_CSTS_READY: u8 = (1 << 0); // Set to 1 when the controller is ready to accept submission queue doorbell writes
const NVME_CSTS_NSSRO: u8 = (1 << 4); // NVM Subsystem reset occurred

const NVME_AQA_AQS_MASK: u64 = 0xfff; // Admin queue size mask
const NVME_AQA_ACQS: fn(u64) -> u64 = |x| (((x) & NVME_AQA_AQS_MASK) << 16); // Admin completion queue size
const NVME_AQA_ASQS: fn(u64) -> u64 = |x| (((x) & NVME_AQA_AQS_MASK) << 0); // Admin submission queue size

// "NVME", initiates a reset
const NVME_NSSR_RESET_VALUE: u64 = 0x4E564D65;
