use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{disk::ata::DiskLoc, fs_driver};

use super::{partition::Partition, userland::FatAttributes};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(packed)]
pub struct BiosParameterBlock {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub fats: u8,
    pub root_entries: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub sectors_per_fat_16: u16,
    pub sectors_per_track: u16,
    pub heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,

    // Extended BIOS Parameter Block
    pub sectors_per_fat_32: u32,
    pub extended_flags: u16,
    pub fs_version: u16,
    pub root_dir_first_cluster: u32,
    pub fs_info_sector: u16,
    pub backup_boot_sector: u16,
    pub reserved_0: [u8; 12],
    pub drive_num: u8,
    pub reserved_1: u8,
    pub ext_sig: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type_label: [u8; 8],
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum FileSystemError {
    FileNotFound,
    CantWrite,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct FilePath {
    raw_path: String,
    pub partition: Partition,
}
impl FilePath {
    pub fn new(mut full_path: String, partition: Partition) -> Self {
        if !full_path.starts_with("/") {
            full_path.insert(0, '/');
        }
        Self {
            raw_path: full_path.replace("\u{ffff}", ""),
            partition,
        }
    }
    pub fn splitted(&self) -> core::str::Split<'_, &str> {
        self.raw_path.split("/")
    }
    pub fn len(&self) -> u64 {
        let mut len = 0;
        for _word in self.splitted() {
            len += 1;
        }
        len
    }
    pub fn root(&self) -> &str {
        let mut splitted = self.splitted();
        splitted.next().unwrap()
    }
    /// Creates a new filepath poiting to parent
    pub fn parent(&self) -> FilePath {
        let splitted: Vec<&str> = self.splitted().collect();
        FilePath::new(splitted[0..splitted.len() - 2].join("/"), self.partition.clone())
    }
    pub fn path(&self) -> &String {
        &self.raw_path
    }
    // Both paths must be on same partition !
    pub fn join(&self, other_path: FilePath) -> FilePath {
        let mut path = self.raw_path.clone();
        assert_eq!(self.partition, other_path.partition);
        path.extend(other_path.path().chars());
        Self::new(path.replace("//", "/").replace("\\", "/"), self.partition.clone())
    }
    //TODO Return new or mutate self ?
    pub fn join_str(&self, other_path: String) -> FilePath {
        let path = &format!("{}/{}", self.path(), other_path);
        Self::new(path.replace("//", "/").replace("\\", "/"), self.partition.clone())
    }
    pub fn disk_loc(&self) -> DiskLoc {
        self.partition.0
    }
    pub fn name(&self) -> &str {
        self.splitted().last().unwrap()
    }
}
impl core::fmt::Debug for FilePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("FilePath {:?}", self.path()).as_str())
    }
}
impl core::fmt::Display for FilePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("{}", self.path()).as_str())
    }
}