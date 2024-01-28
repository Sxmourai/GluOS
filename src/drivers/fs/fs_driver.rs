use core::fmt::Debug;

use alloc::{
    boxed::Box, format, string::{String, ToString}, vec::Vec
};
use hashbrown::HashMap;
use log::{debug, error};
use x86_64::registers::rflags::read;

use super::{
    fat::Fat32Driver, fs::*, partition::Partition, userland::FatAttributes
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

pub trait FsDriver: core::fmt::Debug + FsDriverInitialiser {
    fn read(&self, path: &FilePath) -> Result<Entry, FsReadError>;
    fn read_file(&self,filepath: &FilePath) -> Result<File, FsReadError> {
        match self.read(filepath)? {
            Entry::File(f) => Ok(f),
            Entry::Dir(d) => Err(FsReadError::EntryNotFound),
        }
    }
    fn read_dir(&self, dirpath: &FilePath) -> Result<Dir, FsReadError> {
        match self.read(dirpath)? {
            Entry::File(f) => Err(FsReadError::EntryNotFound),
            Entry::Dir(d) => Ok(d),
        }
    }
    //TODO fn write(&self, path: &FilePath) -> Result<Entry, FsWriteError>;
    //TODO fn write_file(&self,filepath: &FilePath) -> Result<FileEntry, FsWriteError>;
    //TODO fn write_dir (&self, dirpath: &FilePath) -> Result<DirEntry,  FsWriteError>;
    fn as_enum(&self) -> FsDriverEnum;
    fn partition(&self) -> &Partition;
}

pub trait FsDriverInitialiser {
    //TODO Return result
    fn try_init(partition: &Partition) -> Option<Box<Self>> where Self: Sized;
    // fn index_disk(&mut self) {
    //     self.mut_files().extend(self.walk_dir("/"))
    // }
    // /// Transforms a soft entry to a real entry by reading it (if file, reads file contents, if dir reads sub elements (sub elements will be soft))
    // fn soft_hard(&mut self, soft_entry: SoftEntry) -> Entry;
    // /// Performs a soft read of the value (i.e. reads the inode but not the file contents)
    // fn soft_read(&self, entry: FilePath) -> Result<SoftEntry, FsReadError>;
    // fn walk_dir(&self, dir: SoftDir) -> Result<HashMap<FilePath, SoftEntry>, FsReadError> {
    //     let mut files = HashMap::new();
    //     let entries = match self.soft_hard(dir) {
    //         Entry::File(f) => todo!(),
    //         Entry::Dir(d) => d.soft_entries(),
    //     };

    //     for entry in entries {
    //         match entry {
    //             SoftEntry::File(sf) => {files.insert(FilePath::new(format!("{}/{}", entry.path(), entry.name()), self.partition().clone()));},
    //             SoftEntry::Dir(sd) => {files.extend(self.walk_dir(entry)?);}
    //         };
    //     }
    //     Ok(files)
    // }
    // fn mut_files(&mut self) -> &mut HashMap<FilePath, SoftEntry>;
}


#[derive(Debug)]
pub struct SoftEntry {
    pub path: FilePath,
    pub size: usize,
}


//TODO Hold a driver Fat32(Fat32Driver)
pub enum FsDriverEnum {
    Fat32,
    Ext,
    NTFS,
    //NOT SUPPORTED
    BTRFS,
    TFS,
}
impl core::fmt::Display for FsDriverEnum {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let repr = match self {
            Self::Fat32=>"Fat32",
            Self::Ext=>"Ext",
            Self::NTFS=>"Ntfs",
            Self::BTRFS=>"Btrfs",
            Self::TFS=>"Tfs (redox)",
        };
        f.write_str(repr)
    }
}
#[derive(Debug)]
pub enum FsReadError {
    EntryNotFound,
    ReadingDiskError, //TODO This error should come from the ATA errors (see issue better error handling)
    ParsingError
}

#[derive(Debug)]
pub enum Entry {
    File(File),
    Dir(Dir),
}

#[derive(Debug)]
pub struct File {
    pub path: FilePath,
    pub content: String,
    pub size: usize,
}
#[derive(Debug)]
pub struct Dir {
    pub path: FilePath,
    pub entries: Vec<SoftEntry>,
    //TODO Sum of entries size ? For now entries.len
    pub size: usize,
}
