use core::fmt::Debug;

use alloc::{
    boxed::Box, format, string::{String, ToString}, vec::Vec
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
    type Entry;
    type SoftEntry;
    type Files;// = HashMap<FilePath, Entry>; unstable :c
    fn read(&self, path: &FilePath) -> Result<Entry, FsReadError>;
    fn read_file(&self,filepath: &FilePath) -> Result<Box<dyn FileEntry>, FsReadError>;
    fn read_dir (&self, dirpath: &FilePath) -> Result<Box<dyn DirEntry>,  FsReadError>;
    //TODO fn write(&self, path: &FilePath) -> Result<Entry, FsWriteError>;
    //TODO fn write_file(&self,filepath: &FilePath) -> Result<FileEntry, FsWriteError>;
    //TODO fn write_dir (&self, dirpath: &FilePath) -> Result<DirEntry,  FsWriteError>;
    //TODO Return result
    fn try_init(partition: &Partition) -> Option<Box<Self>> where Self: Sized;
    fn index_disk(&mut self) {
        self.mut_files().extend(self.walk_dir("/"))
    }
    /// Performs a soft read of the value (i.e. reads the inode but not the file contents)
    fn soft_read(&self, entry: &Self::Entry) -> Result<dyn SoftEntry, FsReadError>;
    fn walk_dir(&self, entry: Self::SoftEntry) -> Option<Self::Files> {
        let mut files = Self::Files::new();
        for entry in self.soft_read(entry)? {
            match entry.type_indicator() {
                SoftEntry::File(sf) => files.insert(FilePath::new(format!("{}/{}", entry.path(), entry.name()), self.partition().clone())),
                SoftEntry::Dir(sd) => files.extend(self.walk_dir(entry))
            }
        }
        Some(files)
    }
    fn mut_files(&mut self) -> &mut Self::Files;
    fn as_enum(&self) -> FsDriverEnum;
    fn partition(&self) -> &Partition;
}

pub trait SoftEntry {

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
#[derive(Debug)]
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

pub trait FileEntry: Debug {
    // Returns mut because the file could impl caching ?
    fn content(&mut self) -> &String;
    fn size(&self) -> usize;
    fn name(&self) -> &String;
}
pub trait DirEntry: Debug {
    fn entries(&mut self) -> &Vec<Entry>;
    /// Size of all sub elements
    fn size(&self) -> usize;
    fn name(&self) -> &String;
}
