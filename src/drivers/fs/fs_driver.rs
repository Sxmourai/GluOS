use alloc::{
    boxed::Box, string::{String, ToString}, vec::Vec
};
use hashbrown::HashMap;
use log::{debug, error};
use x86_64::registers::rflags::read;

use super::{
    ext::{self, ExtDriver, ExtSuperBlock}, fat::Fat32Driver, fs::*, partition::Partition, userland::FatAttributes
};
use crate::{
    bit_manipulation::any_as_u8_slice,
    dbg,
    disk::{
        ata::{read_from_disk, write_to_disk, DiskLoc, DISK_MANAGER, DiskAddress, read_from_partition, write_to_partition},
        DiskError,
    },
    serial_print, serial_println,
};

pub trait FsDriver: core::fmt::Debug {
    fn read(&self, path: &FilePath) -> Result<Entry, FsReadError>;
    fn read_file(&self,filepath: &FilePath) -> Result<Box<dyn FileEntry>, FsReadError>;
    fn read_dir (&self, dirpath: &FilePath) -> Result<Box<dyn DirEntry>,  FsReadError>;
    //TODO fn write(&self, path: &FilePath) -> Result<Entry, FsWriteError>;
    //TODO fn write_file(&self,filepath: &FilePath) -> Result<FileEntry, FsWriteError>;
    //TODO fn write_dir (&self, dirpath: &FilePath) -> Result<DirEntry,  FsWriteError>;
    fn get_partition(&self, partition_id: u8) -> Option<Partition>;
    //TODO Return result
    fn try_init(partition: &Partition) -> Option<Box<Self>> where Self: Sized;
    fn as_enum(&self) -> FsDriverEnum;
}
//TODO Hold a driver Fat32(Fat32Driver)
pub enum FsDriverEnum {
    Fat32,
    Ext,
}

pub enum FsReadError {
    EntryNotFound,
    ReadingDiskError, //TODO This error should come from the ATA errors (see issue better error handling)
    ParsingError
}
pub enum Entry {
    File(Box<dyn FileEntry>),
    Dir(Box<dyn DirEntry>),
}
impl Entry {
    pub fn name(&self) -> &String {
        match self {
            Entry::File(f) => f.name(),
            Entry::Dir(d) => d.name(),
        }
    }
    pub fn size(&self) -> usize {
        match self {
            Entry::File(f) => f.size(),
            Entry::Dir(d) => d.size(),
        }
    }
}

pub trait FileEntry {
    // Returns mut because the file could impl caching ?
    fn content(&mut self) -> &String;
    fn size(&self) -> usize;
    fn name(&self) -> &String;
}
pub trait DirEntry {
    fn entries(&mut self) -> &Vec<Entry>;
    /// Size of all sub elements
    fn size(&self) -> usize;
    fn name(&self) -> &String;
}
