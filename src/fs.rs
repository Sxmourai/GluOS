use core::num::TryFromIntError;

use alloc::{string::{String, ToString}, vec::Vec};

use crate::{pci::ata::{self, SECTOR_SIZE, SSECTOR_SIZE, Channel, Drive, disk_manager, read_from_disk}, dbg, serial_print_all_bits, serial_print, serial_println};

#[derive(Debug)]
pub enum DiskError {
    NotFound, 
    PermissionDenied, // Shouldn't happen... But keep this for rlib ?
    SectorTooBig,
    NoReadModeAvailable,
    DiskNotFound,
}

impl From<TryFromIntError> for DiskError {
    fn from(original: TryFromIntError) -> Self {
        DiskError::SectorTooBig
    }
}

pub struct File {
    name: String,
    len: u64,
}
impl File {
    pub fn new(name:String) -> Self {
        Self {
            name,
            len: 10,
        }
    }
    pub fn read(&self) -> Result<String, DiskError> {
        let start = 0;
        let content = read_from_disk(1u8, start, start+self.len);
        Ok(content)
    }
    pub fn write(&self, content: String) -> Result<(), DiskError> {
        Ok(())
    }
    pub fn delete(&self) -> Result<(), DiskError> {
        Ok(())
    }
}
// create [--object objectdef] [-q] [-f fmt] [-b backing_file] [-F backing_fmt] [-u] [-o options] filename [size]
pub fn open(filename: &str) -> Result<File, DiskError> {
    Ok(File::new(filename.to_string()))
}
pub fn read(filename: &str) -> Result<String, DiskError> {
    open(filename)?.read()
}
pub fn write(filename: &str, content: &str) -> Result<(), DiskError> {
    open(filename)?.write(content.to_string())
}
pub fn delete(filename: &str) -> Result<(), DiskError> {
    open(filename)?.delete()
}

pub fn unite_sectors(sectors: Vec<[u16;SSECTOR_SIZE]>) -> Vec<u16> {
    let mut united = Vec::new();
    for sector in sectors {
        for word in sector {
            united.push(word);
        }
    }
    united
}



