//! Used wiki.osdev.org/NVMe
//! Used https://github.com/torvalds/linux/blob/master/include/linux/nvme.h
use core::ptr::addr_of;

use alloc::vec::Vec;
use bit_field::BitField;
use x86_64::{structures::paging::{Page, PageTableFlags, PhysFrame}, PhysAddr, VirtAddr};

use crate::{bit_manipulation::{all_zeroes, any_as_u8_slice}, dbg, mem_map, memory::handler::map_frame, pci::PciDevice};

pub fn init(nvme_pci: &PciDevice) -> Option<Vec<NVMeDisk>> {
    log::debug!("{}", nvme_pci);
    let bar0 = nvme_pci.raw.bars[0] as u64 & 0xFFFF_0000;
    // mem_map!(frame_addr=bar0, WRITABLE);
    for i in 0..64 {
        mem_map!(frame_addr=bar0+(0x1000*i), WRITABLE);
    }
    let regs = unsafe{NVMeRegisters::new(bar0 as usize)};
    dbg!(regs);
    regs.controller_status=0;
    // regs.nvm_subsystem_reset = 0x4E564D65; // Reset NVM
    while !regs.ready() {}
    regs.admin_submission_queue = *(regs.base() as u64+0x1000).set_bits(0..11, 0);
    regs.admin_completion_queue = *(regs.base() as u64+0x1000).set_bits(0..11, 0);
    dbg!(regs);
    
    dbg!(regs.capas_doorbell_stride());
    let addr = 4096*390;
    //TODO Does the NVMe needs it ?
    //TODO Get a frame allocator for low memory
    let buffer = unsafe{core::slice::from_raw_parts_mut(addr as *mut u8, 4096)}; // Rolled a dice, I swear it's random 🤣
    mem_map!(frame_addr=addr, WRITABLE);
    let identify = SubmissionEntry::new_identify(IdentifyType::Controller, None, buffer);
    // 
    let submission_addr = regs.admin_submission_queue;
    mem_map!(frame_addr=submission_addr, WRITABLE);
    mem_map!(frame_addr=submission_addr+0x1000, WRITABLE);
    regs.add_submission_entry(identify);
    loop {
        let q = regs.completion_queue();
        // if q.len()>0 {
        //     dbg!(q);
        //     break
        // }
        let buffer = unsafe{core::slice::from_raw_parts(addr as *const u8, 4096)};
        // dbg!(q);
        if !all_zeroes(buffer) {
            break
        }
    }
    Some(Vec::new())
}



pub struct NVMeDisk {
    
}

#[derive(Debug)]
#[repr(C, packed)]
struct NVMeRegisters {
    controller_caps: u64,
    version: u64,
    interrupt_mask_set: u32,
    interrupt_mask_clear: u32,
    controller_config: u32,
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
    /// # Safety
    /// Ensure that bar0 address is the proper base for the NVMe registers
    pub unsafe fn new(bar0: usize) -> &'static mut Self {
        unsafe {&mut *(bar0 as *mut Self)}
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
        for i in 0..1000u64 { // Max queues is 64Kib
            let addr = ((self.base() as u64+0x1000)+(2*i)*self.db_stride());
            let v = unsafe {&*(addr as *const SubmissionEntry)}.clone();
            if all_zeroes(any_as_u8_slice(&v)) {break}
            queue.push(v);
        }
        queue
    }
    ///((self.base() as u64+0x1000)+(2*0+1)*self.db_stride())
    pub fn add_submission_entry(&mut self, entry: SubmissionEntry) {
        unsafe {*(self.admin_submission_queue as *mut SubmissionEntry) = entry}
    }
    pub fn completion_queue(&self) -> Vec<CompletionEntry> {
        // Can we know the size of the vec ? If so with_capacity()
        let mut queue = Vec::new();
        for i in 0..1000u64 { // Max queues is 64Kib
            let v = unsafe {&*(self.admin_completion_queue as *const CompletionEntry)}.clone();
            if all_zeroes(any_as_u8_slice(&v)) {break}
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
    pub fn new_identify(to_identify: IdentifyType, namespace_id: Option<u32>, buffer: &mut [u8]) -> Self {
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
        .field("command_specific", &command_specific).finish()
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
    pub fn new(opcode: u8, fused_op: u8, prp_or_sql_select: u8, command_id:u16) -> Self {
        Self {
            opcode,
            command_id,
            raw: *fused_op.get_bits(0..2).set_bits(6..8, prp_or_sql_select.get_bits(6..8))
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
    IO(IOCommand)
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
    Namespace=0,
    Controller=1,
    NamespaceList=2,
}