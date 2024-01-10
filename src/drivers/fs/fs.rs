use core::num::TryFromIntError;

use alloc::{string::{String, ToString, ParseError}, vec::{Vec, self}, boxed::Box, format};
use hashbrown::HashMap;
use log::error;

use crate::{serial_print, serial_println, println, print, drivers::disk::{ata::{Sectors, DiskLoc, read_from_disk}, DiskError}, state::{get_state, fs_driver}, dbg};

use super::{fs_driver::{self, FsDriver, Files}, userland::FatAttributes};

#[derive(Default, Debug, Clone)]
#[repr(packed)]
pub struct BiosParameterBlock {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8;8],
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
    FileNotFound
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct FilePath {
    raw_path: String,
}
impl FilePath {
    pub fn new(mut full_path: String) -> Self {
        if !full_path.starts_with("/") {
            full_path.insert(0, '/');
        }
        Self {
            raw_path: full_path
        }
    }
    pub fn splitted(&self) -> core::str::Split<'_, &str> {
        self.raw_path.split("/")
    }
    pub fn len(&self) -> u64 {
        let mut len = 0;
        for word in self.splitted() {
            len += 1;
        }
        len
    }
    pub fn root(&self) -> &str {
        let mut splitted = self.splitted();  
        splitted.next().unwrap()
    }
    pub fn parent(&self) -> FilePath {
        let mut splitted:Vec<&str> = self.splitted().collect();
        splitted[0..splitted.len()-2].join("/").into()
    }
    pub fn path(&self) -> &String {
        &self.raw_path
    }
    pub fn join(self, other_path: FilePath) -> FilePath {
        let mut path = self.raw_path;
        path.extend(other_path.path().chars());
        Self::new(path.replace("//", "/").replace("\\", "/"))
    }
    // pub fn open_file(&self) -> Result<Fat32Entry, FileSystemError> {
    //     Fat32Entry::open(self)
    // }
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
impl Into<FilePath> for String {
    fn into(self) -> FilePath {
        FilePath::new(self)
    }
}
impl Into<FilePath> for &str {    
    fn into(self) -> FilePath {
        FilePath::new(self.to_string())
    }
}
#[derive(Debug, Clone)]
pub enum Fat32Entry {
    File(Fat32File),
    Dir(Fat32Dir),
}
#[derive(Debug, Clone)]
pub struct Fat32File {
    pub path: FilePath,
    pub size: u64,
    pub attributes: FatAttributes,
    pub sector: u32,
}
impl Fat32File {
    pub fn path(&self) -> &FilePath {&self.path}
    pub fn name(&self) -> &str {&self.path.name()}
    pub fn sector(&self) -> u32 {self.sector}
    pub fn attributes(&self) -> &FatAttributes {&self.attributes}
}
#[derive(Debug, Clone)]
pub struct Fat32Dir {
    pub path: FilePath,
    pub attributes: FatAttributes,
    pub sector: u32,
    // pub dirs: HashMap<FilePath, Fat32Dir>,
}
impl Fat32Dir {
    pub fn path(&self) -> &FilePath {&self.path}
    pub fn name(&self) -> &str {&self.path.name()}
    pub fn sector(&self) -> u32 {self.sector}
    pub fn attributes(&self) -> &FatAttributes {&self.attributes}
}


//TODO Mult by sectors_per_cluster
// All safely to u32
pub fn cluster_to_sector(cluster_number: u64, first_data_sector: u64) -> u64 {
    ((cluster_number-2))+first_data_sector
}
pub fn sector_to_cluster(sector_number: u64, first_data_sector: u64) -> u64 {
    (sector_number-first_data_sector)+2
}

pub enum FatType {
    ExFat,
    Fat12,
    Fat16,
    Fat32,
}
#[derive(Default, Debug)]
pub struct FatInfo(
    pub BiosParameterBlock,
);
impl FatInfo {
    pub fn first_sector_of_cluster(&self) -> u64 {
        let first_data_sector = self.get_first_data_sector();
        cluster_to_sector(self.0.root_dir_first_cluster as u64, first_data_sector) // , self.0.sectors_per_cluster as u64
    }
    pub fn get_first_data_sector(&self) -> u64 {
        let fat_size = self.get_fat_size();
        let root_dir_sectors = self.get_root_dir_sectors();
        let reserved_sector_count = self.0.reserved_sectors;
        reserved_sector_count as u64 + (self.0.fats as u64 * fat_size as u64) + root_dir_sectors
    }
    pub fn fat_type(&self) -> FatType {
        let total_clusters = self.get_total_clusters();
        if(total_clusters < 4085) {FatType::Fat12}
            else if(total_clusters < 65525){FatType::Fat16}
            else {FatType::Fat32}
    }
    pub fn get_total_clusters(&self) -> u64 {
        self.get_data_sectors() as u64 / self.0.sectors_per_cluster as u64
    }
    pub fn get_data_sectors(&self) -> u64 {
        self.get_total_sectors() as u64 - (self.0.reserved_sectors as u64 + (self.0.fats as u64 * self.get_fat_size() as u64) + self.get_root_dir_sectors()) as u64
    }
    pub fn get_total_sectors(&self) -> u32 {
        if (self.0.total_sectors_16 == 0) {self.0.total_sectors_32} else {self.0.total_sectors_16.into()}
    }
    // Gets fat size in sectors
    pub fn get_fat_size(&self) -> u32 {
        if (self.0.sectors_per_fat_16==0) {self.0.sectors_per_fat_32} else {self.0.sectors_per_fat_16 as u32}
    }
    pub fn get_root_dir_sectors(&self) -> u64 {
        ((self.0.root_entries as u64 * 32 as u64) + (self.0.bytes_per_sector as u64 - 1)) / self.0.bytes_per_sector as u64
    }
    pub fn first_fat_sector(&self) -> u16 {
        self.0.reserved_sectors
    }
}


pub struct FatTable {
    pub size: u32,
    pub first_sector: u16,
    pub last_sector:u16,
    pub last_offset:u16, // u16 even though in range 0..512
    pub last_used_sector: u32,
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum ClusterEnum {
    EndOfChain,
    BadCluster,
    Cluster(u32)
}