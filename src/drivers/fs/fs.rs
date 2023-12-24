use core::num::TryFromIntError;

use alloc::{string::{String, ToString, ParseError}, vec::{Vec, self}, boxed::Box};
use fatfs::IntoStorage;
use hashbrown::HashMap;
use log::error;

use crate::{serial_print_all_bits, serial_print, serial_println, u16_to_u8, println, print, drivers::disk::{ata::{Sectors, DiskLoc, read_from_disk}, DiskError}, state::{get_state, fs_driver}};

use super::fs_driver::{self, FsDriver};

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

pub fn parse_sectors(sectors: &Sectors) -> String {
    let mut content = String::new();
    for b in sectors {
        content.push(*b as char);
    }//String::from_utf16_lossy(&sector).as_str()
    content
}


#[derive(PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum FileSystemError {
    FileNotFound
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum GenericFile {
    Fat32,
    // Fat32 => Fat32File(_)
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Debug)]
pub struct FilePath {
    pub(crate) raw_path: String,
}
impl FilePath {
    pub fn new(full_path: String) -> Self {
        Self {
            raw_path: full_path
        }
    }
    pub fn splitted(&self) -> core::str::Split<'_, &str> {
        self.raw_path.split("/")
    }
    pub fn root(&self) -> &str { 
        let mut splitted = self.splitted();  
        let root = splitted.next().unwrap();
        if root.is_empty() {splitted.next().unwrap()}
        else    {root}
    }
    pub fn open_file(&self) -> Result<GenericFile, FileSystemError> {
        Fat32File::open(self)
    }
    pub fn name(&self) -> &str {
        self.splitted().last().unwrap()
    }
}


#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct FatPermissions(pub u8);
#[derive(PartialEq,Eq, Hash, Debug)]
pub struct FatGroup {
    pub group_name: String,
    pub id: u16,
    pub derived_groups: Vec<u16>,
}
#[derive(PartialEq,Eq, Hash, Debug)]
pub struct FatUser {
    pub username: String,
    pub id: u16,
    pub groups: Vec<u16>,
}
pub fn get_group(id: u16) -> FatGroup {
    FatGroup { group_name: "default".to_string(), id, derived_groups: Vec::new() }
}
pub fn get_user(id: u16) -> FatUser {
    FatUser { username: "Sxmourai".to_string(), id, groups: alloc::vec![1] }
}
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum FatPerson {
    Group(FatGroup),
    User(FatUser),
}
impl FatPerson {
    pub fn new(group: bool, id: u16) -> Self {
        if group {
            Self::Group(get_group(id))
        } else {
            Self::User(get_user(id))
        }
    }
}
#[derive(Default, Debug)] // Proper debug for attributes (flags in self.flags)
pub struct FatAttributes {
    flags: u16,
    permissions: HashMap<FatPerson, FatPermissions>,
}
impl FatAttributes {
    pub fn permissions(&self, group: &FatPerson) -> Option<&FatPermissions> {
        self.permissions.get(group)
    }
}
pub trait Fat32Element: core::fmt::Debug {
    fn path(&self) -> &FilePath;
    fn name(&self) -> &str;
    fn size(&self) -> u64;
    fn attributes(&self) -> &FatAttributes;
}
// #[derive(Debug)]
// pub struct Fat32Dir {
//     path: FilePath,
//     size: u64,
//     attributes: FatAttributes,
// }
// impl Fat32Element for Fat32Dir {
//     fn name(&self) -> &String {&self.name}
//     fn size(&self) -> u64 {self.size}
//     fn attributes(&self) -> &FatAttributes {&self.attributes}
// }
#[derive(Debug)]
pub struct Fat32File {
    path: FilePath,
    size: u64,
    attributes: FatAttributes,
}
impl Fat32Element for Fat32File {
    fn path(&self) -> &FilePath {&self.path}
    fn name(&self) -> &str {&self.path.name()}
    fn size(&self) -> u64 {self.size}
    fn attributes(&self) -> &FatAttributes {&self.attributes}
}
impl Fat32File {
    pub fn new(path: FilePath, size: u64, attributes: FatAttributes) -> Self {
        Self {
            path,
            size,
            attributes,
        }
    }
    pub fn open(path: &FilePath) -> Result<GenericFile, FileSystemError> {
        get_state().fs().lock().open_file(path)
    }
    pub fn close(&mut self) -> Result<(), FileSystemError> {
        get_state().fs().lock().close_file(self)
    }
}
// impl Fat32File {
//     pub fn open(path: &FilePath) -> Result<GenericFile, FileSystemError> {
//         get_state().fs().lock().open_file(path)
//     }
//     pub fn close(&mut self) -> Result<(), FileSystemError> {
//         get_state().fs().lock().close_file(self)
//     }
// }


pub enum Elements {
    File(Fat32File),
    Dir(Fat32File),
    Other(Box<dyn Fat32Element>),
}


#[derive(Debug, Clone)]
pub struct FatEntry {
    pub sector: u64,
    pub is_file: bool
}
impl FatEntry {
    pub fn new(sector: u64, is_file: bool) -> Self {
        Self {
            sector,
            is_file,
        }
    }
}


pub fn cluster_to_sector(cluster_number: u64, first_data_sector: u64) -> u64 {
    (cluster_number-2)+first_data_sector
}
pub fn sector_to_cluster(sector_number: u64, first_data_sector: u64) -> u64 {
    (sector_number-first_data_sector)+2
}