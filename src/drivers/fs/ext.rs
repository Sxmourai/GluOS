use core::str::FromStr;

use alloc::{boxed::Box, string::{String, ToString}, vec::Vec};
use bytemuck::Zeroable;
use hashbrown::HashMap;

use crate::{dbg, disk::ata::read_from_partition, bit_manipulation::{all_zeroes, any_as_u8_slice}};

use super::{path::FilePath, fs_driver::{Dir, Entry, File, FsDriver, FsDriverEnum, FsDriverInitialiser, FsReadError, SoftEntry}, partition::Partition};

pub struct InodeNumber(u64);
#[derive(Debug)]
pub struct ExtDriver{
    partition: Partition,
    ///TODO Know how to handle the superblock (if it's extended or not)
    superblock: Superblock,
    ///TODO Know how to handle the block group descriptors (if it's 64 or 32 bytes) (we can use a "combinator" that combines 2 block_group_descriptors into a 64 bytes one)
    blk_grp_desc_table: Vec<BlockGroupDescriptor>,
    //TODO Hold a cached root info
    files: HashMap<FilePath, ExtEntryDescriptor>
}
impl ExtDriver {
    fn extsuperblock(&self) -> &ExtendedExtSuperblock {
        self.superblock.as_ext_super_block()
    }
    fn superblock(&self) -> &ExtSuperBlock {
        self.superblock.as_super_block()
    }
    fn block_size(&self) -> u32 {
        self.superblock().block_size()/256
    }
    fn walk_dir(&self, dir: &ExtEntryDescriptor) -> Result<HashMap<FilePath, ExtEntryDescriptor>, FsReadError> {
        let mut files = HashMap::new();

        let inode = self.get_inode(dir.inner.inode).ok_or(FsReadError::EntryNotFound)?;
        let entries = match self.read_inode_block(inode, dir)?  {
            ExtEntry::File(f) => {return Err(FsReadError::EntryNotFound)},
            ExtEntry::Dir(d) => {
                d.entries
            },
        };
        for entry in entries {
            files.insert(FilePath::new(entry.name.clone(), self.partition.clone()), entry);
        }

        Ok(files)
    }
    fn index_disk(&mut self) {
        let root = ExtEntryDescriptor::new_raw(2, "/".to_string(), false);
        self.files = self.walk_dir(&root).unwrap();
        self.files.insert(FilePath::new("/".to_string(), self.partition.clone()), root);
    }
    fn dir_entries_contain_type(&self) -> bool {
        self.extsuperblock().required_feat_present & 0x2!=0
    }
    fn read_inode(&self, inode: Inode) -> Option<ExtEntryDescriptor> {
        let block_size = self.block_size();
        let data_blk = read_from_partition(&self.partition, (inode.direct_blk_ptr_0*block_size) as u64, block_size.try_into().unwrap()).ok()?;
        let inode_entry = ExtEntryDescriptor::new(&data_blk, self.dir_entries_contain_type());
        Some(inode_entry)
    }
    /// Takes a inode number and returns the inode
    fn get_inode(&self, inode_number: u32) -> Option<Inode> {
        let blk_grp_number = block_group_of_inode(inode_number as u64, self.superblock().inodes_per_group);
        let blk_grp = &self.blk_grp_desc_table[blk_grp_number as usize];
        let block_size = self.block_size();
        let inode_table_start_sector = (blk_grp.lo_block_addr_of_inode_start*block_size) as u64;
        let inode_size = self.extsuperblock().size_inode_struct as usize;
        let inode_table_start_sector = inode_table_start_sector+((inode_number as u64-1)/(512/inode_size as u64));
        let tables = get_inode_table(&self.partition, inode_table_start_sector, inode_size).unwrap();
        let inode = &tables[(inode_number as usize-1)%(512/inode_size)];
        Some(inode.clone())
    }
    fn read_inode_block(&self, inode: Inode, entry: &ExtEntryDescriptor) -> Result<ExtEntry, super::fs_driver::FsReadError> {
        let data_blk = read_from_partition(self.partition(), (inode.direct_blk_ptr_0*self.block_size()) as u64, self.block_size() as u16).or(Err(FsReadError::ReadingDiskError))?;
        if inode.type_n_perms&0x4000==0x4000 { //DIR
            let mut idx = 0; // usize cuz slice indexing
            let mut entries = Vec::new();
            loop {
                let sl = &data_blk[idx..];
                if sl.len()<=12 || all_zeroes(&sl[..12]) {
                    break
                };
                let ext_entry = ExtEntryDescriptor::new(sl, self.dir_entries_contain_type());
                idx+=ext_entry.inner.entry_size as usize;
                // let entry = match ext_entry.type_indicator() {
                //     ExtInodeType::File => ExtEntryDescriptor { inner: (), name: () },
                //     ExtInodeType::Dir => ExtEntryDescriptor { inner: (), name: () },
                //     _ => todo!(),
                // };
                entries.push(ext_entry);
            };
            Ok(ExtEntry::Dir(ExtDir { path: FilePath::new(entry.name.clone(), self.partition.clone()), inode: entry.inner.inode, size: inode.lo_32b_size as u64, type_indicator: entry.type_indicator(), entries }))

        } else if inode.type_n_perms&0x8000==0x8000 { //FILE
            let content = String::from_utf8_lossy(&data_blk).to_string();
            // Ok(Entry::File(File {
            //     // inner: inode_entry,
            //     content,
            //     path: FilePath::new(entry.name.clone(), self.partition),
            //     size: inode.lo_32b_size as usize, //TODO Combine with hi_32b_size if 64bit feature ste
            // }))
            Ok(ExtEntry::File(ExtFile { path: FilePath::new(entry.name.clone(), self.partition.clone()), inode: entry.inner.inode, size: inode.lo_32b_size as u64, type_indicator: entry.type_indicator(), content: String::from_utf8_lossy(&data_blk).to_string() }))
        } else {
            let typ = inode.type_n_perms;
            log::error!("Unknown inode type: {:b}", typ);
            dbg!(inode);
            Err(FsReadError::ParsingError)
        }
    }
}

impl FsDriver for ExtDriver {
    fn read(&self, path: &FilePath) -> Result<super::fs_driver::Entry, super::fs_driver::FsReadError> {
        let entry = self.files.get(path).ok_or(FsReadError::EntryNotFound)?;
        let inode = self.get_inode(entry.inner.inode).ok_or(FsReadError::EntryNotFound)?;
        match self.read_inode_block(inode, entry)? {
            ExtEntry::Dir(d) => {
                let mut entries = Vec::with_capacity(d.entries.len());
                for entry in d.entries {
                    entries.push(SoftEntry { path: FilePath::new(entry.name, self.partition().clone()), size: 0 })
                }
                Ok(Entry::Dir(Dir { path: d.path, entries, size: d.size as usize }))
            },
            ExtEntry::File(f) => {
                Ok(Entry::File(File { path: f.path, content: f.content, size: f.size as usize }))
                
            },
        }
    }
    fn as_enum(&self) -> FsDriverEnum {FsDriverEnum::Ext}
    fn partition(&self) -> &Partition {&self.partition}
}
impl FsDriverInitialiser for ExtDriver {
    fn try_init(partition: &Partition) -> Option<Box<Self>> where Self: Sized {
        let superblock = read_superblock(partition)?;
        let extsuperblock = match superblock.as_super_block().major_portion_version {
            0 => {
                log::error!("Superblock version is to old, don't support it {:?} {:?}", superblock.as_super_block(), partition); 
                return None
            },
            _ => {
                superblock.as_ext_super_block()   
            }
        };
        if check_superblock(extsuperblock) {
            log::error!("Error in superblock, probably something not supported !");
            return None
        }
        let bg_size = extsuperblock.block_descriptor_group_size() as usize;
        if bg_size == 64 {
            log::error!("Ext4 not currently supported sry !");
            return None
        }
        let block_size = extsuperblock.super_block.block_size()/256;
        let raw_bgdt = read_bgdt(partition, block_size/2);
        let mut bgds = Vec::new();
        for raw_bgd in raw_bgdt.chunks_exact(32) { //TODO Support 64 bit mode for ext4 i.e.
            if raw_bgd.iter().all(|x|*x==0) {break}
            let bgd = unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor) }.clone();
            bgds.push(bgd);
        }
        let mut _self = Self {
            partition: partition.clone(),
            superblock,
            blk_grp_desc_table: bgds,
            files: HashMap::new(),
        };
        _self.index_disk();
        Some(Box::new(_self))
    }
}

pub enum ExtEntry {
    Dir(ExtDir),
    File(ExtFile),
}
pub struct ExtDir {
    path: FilePath,
    inode: u32,
    size: u64,
    type_indicator: ExtInodeType,
    entries: Vec<ExtEntryDescriptor>,
}
pub struct ExtFile {
    path: FilePath,
    inode: u32,
    size: u64,
    type_indicator: ExtInodeType,
    content: String,
}


//Block size in sectors
//TODO Make some parsing ?
fn read_bgdt(partition: &Partition, block_size: u32) -> Vec<u8> {
    
    read_from_partition(partition, ((block_size)*2).into(), 1).expect("Failed reading Block Group Descriptor")
}
//TODO Not use vec but [Inode; 4]
/// inode_size should be u16
fn get_inode_table(partition: &Partition, inode_table_start_sector: u64, inode_size: usize) -> Option<Vec<Inode>> {
    let raw_inode_table = read_from_partition(partition, inode_table_start_sector, 1).unwrap();
    let mut tables = Vec::new();
    for i in 0..raw_inode_table.len()/inode_size {
        let inode_table = unsafe { &*(raw_inode_table[i*inode_size..].as_ptr() as *const Inode) }.clone();
        tables.push(inode_table);
    }
    Some(tables)
}

fn read_superblock(partition: &Partition) -> Option<Superblock> {
    let mut rawsuper_block = read_from_partition(partition, 2, 2).expect("Failed reading partition on disk");
    if all_zeroes(&rawsuper_block) {return None}
    let superblock = Superblock::new(rawsuper_block);
    Some(superblock)
}

pub struct Superblock {
    /// Must len be >1024
    data: Vec<u8>,
}

impl Superblock {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn as_super_block(&self) -> &ExtSuperBlock {
        bytemuck::from_bytes(&self.data[..core::mem::size_of::<ExtSuperBlock>()])
    }
    pub fn as_ext_super_block(&self) -> &ExtendedExtSuperblock {
        bytemuck::from_bytes(&self.data[..core::mem::size_of::<ExtendedExtSuperblock>()])
    }
}
impl core::fmt::Debug for Superblock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.as_super_block().major_portion_version>=1 {
            f.debug_struct("Superblock").field("data", &self.as_ext_super_block()).finish()
        } else {
            f.debug_struct("Superblock").field("data", &self.as_super_block()).finish()
        }
    }
}

// Returns true if error in superblock (i.e. smth not supported)
//TODO Return Result<()
fn check_superblock(extsuperblock: &ExtendedExtSuperblock) -> bool {
    let n_b_gs = extsuperblock.super_block.total_blocks.div_ceil(extsuperblock.super_block.blocks_per_group);
    let n_b_gs_i = extsuperblock.super_block.total_inodes.div_ceil(extsuperblock.super_block.inodes_per_group);
    if n_b_gs != n_b_gs_i {
        return true;
    }
    if extsuperblock.required_feat_present&0x1!=0 {
        log::error!("Compression used !");
        return true;
    }
    false
}
fn block_group_of_inode(inode_number: u64, inodes_per_group: u32) -> u64 {
    (inode_number - 1) / inodes_per_group as u64
}
fn get_blkgrp64(block_group: u64, bgd: &[u8]) -> Option<&BlockGroupDescriptor64> {
    let raw_bgd = bgd.chunks_exact(64).nth(block_group as usize)?;
    Some(unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor64) })
}
// fn get_inode(inode_number: u64) -> String {
    
// TODO When we will impl ext4
//* } else if bg_size==64 {
    // let bgd = unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor64) };
    // let inode_table_sec = (bgd.inode_table_addr()+1)*(block_size as u64/512);
    // dbg!(inode_table_sec);
    // let raw_inode_tables = read_from_partition(&partition, 109, 2).expect("Failed reading inode table");
    // for raw_table in raw_inode_tables.chunks_exact(128) {
    //     // if raw_table.iter().all(|x|*x==0) {continue}
    //     let inode_table = unsafe { &*(raw_table.as_ptr() as *const Inode) };
    //     if inode_table.type_n_perms&0x4000!=0 {
    //         dbg!("Dir");    
    //     }
    //     if inode_table.type_n_perms&0x8000!=0 {
    //         dbg!("File");
    //     }
    //     dbg!(inode_table);
    // }
// *
// * 
///


#[repr(C, packed)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ExtSuperBlock {
    pub total_inodes: u32,              // Total number of inodes in the file system
    pub total_blocks: u32,              // Total number of blocks in the file system
    pub reserved_blocks: u32,           // Number of blocks reserved for superuser
    pub unallocated_blocks: u32,        // Total number of unallocated blocks
    pub unallocated_inodes: u32,        // Total number of unallocated inodes
    pub superblock_block_number: u32,   // Block number of the block containing the superblock
    pub block_size_shift: u32,          // log2 (block size) - 10
    pub fragment_size_shift: u32,       // log2 (fragment size) - 10
    pub blocks_per_group: u32,          // Number of blocks in each block group
    pub fragments_per_group: u32,       // Number of fragments in each block group
    pub inodes_per_group: u32,          // Number of inodes in each block group
    pub last_mount_time: u32,           // Last mount time (in POSIX time)
    pub last_written_time: u32,
    pub times_mounted_before_consistency_check: u16,
    pub mounts_allowed_before_consistency_check:u16,
    pub ext2_signature: u16, // 0xef53
    pub fs_state: u16, // https://wiki.osdev.org/Ext2#File_System_States (clean / errors)
    pub to_do_when_error: u16, //https://wiki.osdev.org/Ext2#Error_Handling_Methods (Ignore / Remount fs as rd_only / Kpanic)
    pub minor_portion_version: u16, // Combine with full Majour portion (see below)
    pub posix_time_last_consist_chck: u32,
    pub interval_posix_between_forced_consist_chck: u32,
    pub os_id_from_volume_creation: u32, // Linux, GNU HURD, MASIX, FreeBSD, OtherBSDs
    pub major_portion_version: u32,
    pub user_id_can_use_reserved: u16,
    pub group_id_can_use_reserved: u16,
}
impl ExtSuperBlock {
    pub fn block_size(&self) -> u32 {
        self.block_size_shift<<10 // Shifts it 1024
    }
    pub fn fragment_size(&self) -> u32 {
        self.fragment_size_shift<<10
    }
}

#[repr(C, packed)]
#[derive(Copy, Clone)]
pub struct ExtendedExtSuperblock {
    pub super_block: ExtSuperBlock,
    pub fst_non_reserved_inode: u32,
    pub size_inode_struct:u16, // In bytes
    pub block_group_superblock_part_of: u16, // If backup copy
    pub opt_feat: u32, // https://wiki.osdev.org/Ext2#Optional_Feature_Flags
    pub required_feat_present: u32, // required to be supported to read or write https://wiki.osdev.org/Ext2#Required_Feature_Flags
    pub feat_read_only_not_supported: u32, // https://wiki.osdev.org/Ext2#Read-Only_Feature_Flags
    pub fs_id: u128,
    pub volume_name: [u8; 16], // C-style string
    pub path_vol_last_mnt: [u8; 64], // C-style string
    pub compress_algo_used: u32, // See required features
    pub n_blocks_prealloc_for_files: u8,
    pub n_blocks_prealloc_for_dirs: u8,
    pub unused: u16,
    pub journal_id: u128,
    pub journal_inode: u32,
    pub journal_device: u32,
    pub head_orphan_inode_list: u32,
    pub htree_hash_seed_array_32bit_ints: u128,
    pub hash_algo_for_dirs: u8,
    pub journal_blocks_fied_contains_copy_inode_block_array_size: u8,
    pub size_group_descriptors_bytes_in_64bit_mode: u16,
    pub mount_opts: u32,
    pub fst_metablock_block_group: u32, // if enabled
    pub fs_creation_time: u32,
    pub journal_inode_backup_array_32bit_integers: [u8; 68],
    // ! ONLY if 64bit feature is set
    pub e4_hi_total_n_blocks: u32,
    pub e4_hi_total_n_reserved_blocks: u32,
    pub e4_hi_total_n_unallocated_blocks: u32,
    pub e4_min_inode_size: u16,
    pub e4_min_inode_reservation_size: u16,
    pub e4_misc_flags: u32,
    pub e4_log_blocks_rw_perdisk_in_raid_array: u16,
    pub e4_n_secs_wait_in_mmp_check: u16,
    pub e4_block_multi_mount_prevent:u64,
    pub e4_n_block_rw_before_returning_current_disk_raid_array: u32, //amount of disks * stride
    pub e4_n_flex_groups: u8,
    pub e4_meta_checksum_algo_used: u8, //Linux only CRC32
    pub e4_encryption_version_lvl: u8,
    pub e4_reserved_padding:u8,
    pub e4_n_kilo_written_over_fs_lifetime:u64,
    pub e4_inode_n_of_active_snapshot:u32,
    pub e4_sequential_id_active_snapshot:u32,
    pub e4_n_block_reserved_active_snapshot:u64,
    pub e4_inode_number_of_head_of_disk_snapshot_list:u32,
    //TODO rest of fields
    
}
impl ExtendedExtSuperblock {
    pub fn block_descriptor_group_size(&self) -> u8 {
        if self.required_feat_present&0x80==0x80 { // Fs uses 64 bit features
            64
        } else {32}
    }
    pub fn flex_blocks(&self) -> u32 {
        (self.e4_n_flex_groups as u32)<<10
    }
}
unsafe impl bytemuck::Zeroable for ExtendedExtSuperblock {
    fn zeroed() -> Self {
        unsafe { core::mem::zeroed() }
      }
}
unsafe impl bytemuck::Pod for ExtendedExtSuperblock {

}

impl core::fmt::Debug for ExtendedExtSuperblock {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let fst_non_reserved_inode = self.fst_non_reserved_inode; // THX chatgpt
        let size_inode_struct = self.size_inode_struct;
        let block_group_superblock_part_of = self.block_group_superblock_part_of;
        let opt_feat = self.opt_feat;
        let required_feat_present = self.required_feat_present;
        let feat_read_only_not_supported = self.feat_read_only_not_supported;
        let fs_id = self.fs_id;
        let volume_name = self.volume_name;
        let path_vol_last_mnt = self.path_vol_last_mnt;
        let compress_algo_used = self.compress_algo_used;
        let n_blocks_prealloc_for_files = self.n_blocks_prealloc_for_files;
        let n_blocks_prealloc_for_dirs = self.n_blocks_prealloc_for_dirs;
        let unused = self.unused;
        let journal_id = self.journal_id;
        let journal_inode = self.journal_inode;
        let journal_device = self.journal_device;
        let head_orphan_inode_list = self.head_orphan_inode_list;
        f.debug_struct("ExtendedExtSuperblock")
        .field("super_block", &self.super_block)
        .field("fst_non_reserved_inode", &fst_non_reserved_inode)
        .field("size_inode_struct", &size_inode_struct)
        .field("block_group_superblock_part_of", &block_group_superblock_part_of)
        .field("opt_feat", &opt_feat)
        .field("required_feat_present", &required_feat_present)
        .field("feat_read_only_not_supported", &feat_read_only_not_supported)
        .field("fs_id", &fs_id)
        .field("volume_name", &volume_name)
        .field("path_vol_last_mnt", &path_vol_last_mnt)
        .field("compress_algo_used", &compress_algo_used)
        .field("n_blocks_prealloc_for_files", &n_blocks_prealloc_for_files)
        .field("n_blocks_prealloc_for_dirs", &n_blocks_prealloc_for_dirs)
        .field("unused", &unused)
        .field("journal_id", &journal_id)
        .field("journal_inode", &journal_inode)
        .field("journal_device", &journal_device)
        .field("head_orphan_inode_list", &head_orphan_inode_list)
        .finish()
    }
}

#[derive(Debug, Clone, Hash)]
#[repr(C, packed)]
struct RawExtEntryDescriptor {
    pub inode: u32,
    pub entry_size: u16, // including subfields
    pub name_length: u8, // least-significant 8bits
    /// (only if the feature bit for "directory entries have file type byte" is set, else this is the most-significant 8 bits of the Name Length)
    type_indicator: u8, // https://wiki.osdev.org/Ext2#Directory_Entry_Type_Indicators
}

#[derive(Debug, Hash)]
pub struct ExtEntryDescriptor {
    inner: RawExtEntryDescriptor,
    name: String,// Name characters size: N
}
impl<'a> ExtEntryDescriptor {
    /// SAFETY: data.len() > sizeof::<Self>
    /// Tuple is: (self, name)
    pub fn new(data: &'a [u8], dir_entries_contain_type: bool) -> Self {
        let _self = unsafe {&*(data.as_ptr() as *const RawExtEntryDescriptor)};
        let mut name = String::new();
        let mut len = core::mem::size_of::<RawExtEntryDescriptor>()+(_self.name_length as usize);
        if !dir_entries_contain_type {
            len += _self.type_indicator as usize;
        }
        for chr in &data[core::mem::size_of::<RawExtEntryDescriptor>()..len] {
            name.push(*chr as char);
        }
        Self {
            inner: _self.clone(),
            name
        }
    }
    /// Name.len() u8
    pub fn new_raw(inode_number: u32, name: String, file: bool) -> Self {
        let type_indicator = if file {1} else {2};
        Self {
            inner: RawExtEntryDescriptor { inode: inode_number, entry_size: 8+name.len() as u16, name_length: name.len() as u8, type_indicator },
            name,
        }
    }

    pub fn type_indicator(&self) -> ExtInodeType {
        match self.inner.type_indicator {
            0 => ExtInodeType::Unknown,
            1 => ExtInodeType::File,
            2 => ExtInodeType::Dir,
            3 => ExtInodeType::ChrDevice,
            4 => ExtInodeType::BlockDevice,
            5 => ExtInodeType::FIFO,
            6 => ExtInodeType::Socket,
            7 => ExtInodeType::SoftLink,
            _ => {
                log::error!("Read a wrong type from disk !");
                ExtInodeType::Unknown
            }
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExtInodeType {
	Unknown=0,
	File=1,
	Dir=2,
	ChrDevice=3,
	BlockDevice=4,
	FIFO=5,
	Socket=6,
	SoftLink=7,

}

pub enum OptionalFeaturesFlagsExt2 {
    PreallocateNBlocksToDirOnDirCreate=1,
    AFSInodesExist=2,
    FsHasJournal=4, // Ext3 ?
    InodesExtendedAttrs=8,
    FsResizeLargerPartitions=10,
    DirsHashIndex=20,
}
pub enum RequiredFeaturesFlagsExt2 {
    CompressionUsed=1,
    DirsContainTypeField=2,
    FsNeedsReplayJournal=4,
    FsUsesJournalDevice=8,
}
pub enum ReadOnlyFeaturesFlagsExt2 {
    SparseSuperblocksNGroupDescriptorTables=1,
    Fs64bitFileSize=2,
    DirContentBinaryTree=4,
}
#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct BlockGroupDescriptor {
    pub lo_block_addr_block: u32,
    pub lo_block_addr_inode: u32,
    pub lo_block_addr_of_inode_start: u32,
    pub lo_unallocated_blocks_in_group: u16,
    pub lo_unallocated_inodes_in_group: u16,
    pub lo_n_dirs_in_grp: u16,
    pub blk_feat_present: u16,
    pub lo_blk_addr_snapshot_exclude_bitmap: u32,
    pub lo_chksum_blk_usage_bitmap: u16,
    pub lo_chksum_inode_usage_bitmap: u16,
    pub lo_amount_free_inodes: u16, // This allows us to optimize inode searching
    pub chksum_blk_grp: u16, // CRC16
}
#[repr(C, packed)]
pub struct BlockGroupDescriptor64 {
    pub block_group: BlockGroupDescriptor,
    pub hi_block_addr_block: u32,
    pub hi_block_addr_inode: u32,
    pub hi_block_addr_of_inode_table_start: u32,
    pub hi_unallocated_blocks_in_group: u16,
    pub hi_unallocated_inodes_in_group: u16,
    pub hi_n_dirs_in_grp: u16,
    pub hi_amount_free_inodes: u16,
    pub hi_blk_addr_snapshot_exclude_bitmap: u32,
    pub hi_chksum_blk_usage_bitmap: u16,
    pub hi_chksum_inode_usage_bitmap: u16,
    pub reserved: u32, // Reserved in linux
}
impl BlockGroupDescriptor64 {
    pub fn inode_table_addr(&self) -> u64 {
        self.block_group.lo_block_addr_inode as u64 | ((self.hi_block_addr_inode as u64)<<32)
    }
}
impl core::fmt::Debug for BlockGroupDescriptor64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let blk_group = &self.block_group;
        let hi_block_addr_block = self.hi_block_addr_block;
        let hi_block_addr_inode = self.hi_block_addr_inode;
        let hi_block_addr_of_inode_table_start = self.hi_block_addr_of_inode_table_start;
        let hi_unallocated_blocks_in_group = self.hi_unallocated_blocks_in_group;
        let hi_unallocated_inodes_in_group = self.hi_unallocated_inodes_in_group;
        let hi_n_dirs_in_grp = self.hi_n_dirs_in_grp;
        let hi_amount_free_inodes = self.hi_amount_free_inodes;
        let hi_blk_addr_snapshot_exclude_bitmap = self.hi_blk_addr_snapshot_exclude_bitmap;
        let hi_chksum_blk_usage_bitmap = self.hi_chksum_blk_usage_bitmap;
        let hi_chksum_inode_usage_bitmap = self.hi_chksum_inode_usage_bitmap;
        let reserved = self.reserved;
        f.debug_struct("BlockGroupDescriptor64")
        .field("block_group", &blk_group)
        .field("hi_block_addr_block", &hi_block_addr_block)
        .field("hi_block_addr_inode", &hi_block_addr_inode)
        .field("hi_block_addr_of_inode_table_start", &hi_block_addr_of_inode_table_start)
        .field("hi_unallocated_blocks_in_group", &hi_unallocated_blocks_in_group)
        .field("hi_unallocated_inodes_in_group", &hi_unallocated_inodes_in_group)
        .field("hi_n_dirs_in_grp", &hi_n_dirs_in_grp)
        .field("hi_amount_free_inodes", &hi_amount_free_inodes)
        .field("hi_blk_addr_snapshot_exclude_bitmap", &hi_blk_addr_snapshot_exclude_bitmap)
        .field("hi_chksum_blk_usage_bitmap", &hi_chksum_blk_usage_bitmap)
        .field("hi_chksum_inode_usage_bitmap", &hi_chksum_inode_usage_bitmap)
        .field("reserved", &reserved).finish()
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct Inode {
    pub type_n_perms: u16, // https://wiki.osdev.org/Ext2#Inode_Type_and_Permissions
    pub user_id: u16,
    pub lo_32b_size: u32,
    pub last_access_time: u32, // POSIX TIME
    pub creation_time: u32, // POSIX TIME
    pub last_modif_time: u32, // POSIX TIME
    pub deletion_time: u32, // POSIX TIME
    pub group_id: u16,
    pub n_hardlinks_to_inode: u16, // When this reaches 0, the data blocks are marked as unallocated
    pub n_disk_sectors: u32, // not counting the actual inode structure nor directory entries linking to the inode
    pub flags: u32, // https://wiki.osdev.org/Ext2#Inode_Flags
    pub os_specific_1: u32, // https://wiki.osdev.org/Ext2#OS_Specific_Value_1
    pub direct_blk_ptr_0: u32,
    pub direct_blk_ptr_1: u32,
    pub direct_blk_ptr_2: u32,
    pub direct_blk_ptr_3: u32,
    pub direct_blk_ptr_4: u32,
    pub direct_blk_ptr_5: u32,
    pub direct_blk_ptr_6: u32,
    pub direct_blk_ptr_7: u32,
    pub direct_blk_ptr_8: u32,
    pub direct_blk_ptr_9: u32,
    pub direct_blk_ptr_10: u32,
    pub direct_blk_ptr_11: u32,
    pub single_indirect_blk_ptr: u32, // Points to a block that is a list of block pointers to data
    pub double_indirect_blk_ptr: u32, // Points to a block that is a list of block pointers to Singly Indirect Blocks
    pub triple_indirect_blk_ptr: u32, // Points to a block that is a list of block pointers to Doubly Indirect Blocks
    pub gen_number: u32, // Primarily used for NFS
    pub ext_attr_blk: u32, // In Ext2 version 0, this field is reserved. In version >= 1, Extended attribute block (File ACL)
    pub hi_32b_size: u32, // In Ext2 version 0, this field is reserved. In version >= 1, Upper 32 bits of file size (if feature bit set) if it's a file, Directory ACL if it's a directory
    pub blk_addr_frag: u32,
    pub os_spec_2: [u8; 12], // https://wiki.osdev.org/Ext2#OS_Specific_Value_2
}