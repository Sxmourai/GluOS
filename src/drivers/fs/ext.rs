use alloc::string::String;

use crate::{dbg, disk::ata::read_from_partition};

use super::fs_driver::Partition;

pub fn read_root(partition: &Partition) {
    let rawsuper_block = read_from_partition(&partition, 2, 2).expect("Failed reading partition on disk");
    let superblock = unsafe { &*(rawsuper_block.as_ptr() as *const ExtSuperBlock) };
    if superblock.major_portion_version<1 {log::error!("Superblock version is to old, don't support it"); return}
    let extsuperblock = unsafe { &*(rawsuper_block.as_ptr() as *const ExtendedExtSuperblock) };
    // let n_b_gs = extsuperblock.super_block.total_blocks.div_ceil(extsuperblock.super_block.blocks_per_group);
    // let n_b_gs = extsuperblock.super_block.total_inodes.div_ceil(extsuperblock.super_block.inodes_per_group);
    // dbg!(extsuperblock.flex_blocks());
    if extsuperblock.required_feat_present&0x1!=0 {
        log::error!("Compression used !");
        return
    }
    let block_size = extsuperblock.super_block.block_size();
    let inode_size = extsuperblock.size_inode_struct as u32;
    let bg_size = extsuperblock.block_descriptor_group_size() as usize;
    
    let raw_bgdt = read_from_partition(&partition, ((block_size/512)*2).into(), 1).expect("Failed reading Block Group Descriptor");
    let root_blkgrp = block_group_of_inode(2, superblock.inodes_per_group);
    let root_bgd = get_blkgrp64(root_blkgrp, &raw_bgdt).unwrap();
    dbg!(root_bgd.inode_table_addr());
    let inode_table_sec = (root_bgd.inode_table_addr()+1)*(block_size as u64/512);
    // dbg!(inode_table_sec);
    // let root_indtable = read_from_partition(partition, inode_table_sec, 1).unwrap();
    // dbg!(unsafe { &*(root_indtable.as_ptr() as *const InodeTable) });
    
    
    for raw_bgd in raw_bgdt.chunks_exact(bg_size) {
        if raw_bgd.iter().all(|x|*x==0) {break}
        if bg_size==32 {
            let bgd = unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor) };
            let start_inode = (bgd.lo_block_addr_of_inode_start)*4; // Add 2 because first 2 sectors they are empty and superblock starts at 2 and block 0 is at offset 1024 bytes ?
            let root_inode = start_inode as u64+2;
            let raw_inode_table = read_from_partition(&partition, root_inode, 1).unwrap();
            let inode_table = unsafe { &*(raw_inode_table.as_ptr() as *const InodeTable) };
            dbg!("a", inode_table);
        } else if bg_size==64 {
            let bgd = unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor64) };
            let inode_table_sec = (bgd.inode_table_addr()+1)*(block_size as u64/512);
            dbg!(inode_table_sec);
            let raw_inode_tables = read_from_partition(&partition, 109, 2).expect("Failed reading inode table");
            for raw_table in raw_inode_tables.chunks_exact(128) {
                // if raw_table.iter().all(|x|*x==0) {continue}
                let inode_table = unsafe { &*(raw_table.as_ptr() as *const InodeTable) };
                if inode_table.type_n_perms&0x4000!=0 {
                    dbg!("Dir");    
                }
                if inode_table.type_n_perms&0x8000!=0 {
                    dbg!("File");
                }
                dbg!(inode_table);
            }
        }
    }
    let root_inode = 2;
    let root_bg = (root_inode-1) / superblock.inodes_per_group;
    let root_idx = (root_inode-1) % superblock.inodes_per_group;
    let root_blk = (root_idx * inode_size) / block_size;
    let root_sec = root_blk * (block_size / 512);
    dbg!(root_sec, root_bg, root_idx);
}

fn block_group_of_inode(inode_number: u64, inodes_per_group: u32) -> u64 {
    (inode_number - 1) / inodes_per_group as u64
}
fn get_blkgrp64<'a>(block_group: u64, bgd: &'a [u8]) -> Option<&'a BlockGroupDescriptor64> {
    let raw_bgd = bgd.chunks_exact(64).nth(block_group as usize)?;
    Some(unsafe { &*(raw_bgd.as_ptr() as *const BlockGroupDescriptor64) })
}
// fn read_inode(inode_number: u64) -> String {
    
//     String::new()
// }


#[repr(C, packed)]
#[derive(Debug)]
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
#[derive(Debug)]
pub struct InodeTable {
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