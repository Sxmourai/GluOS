use core::num::TryFromIntError;

use alloc::{string::{String, ToString}, vec::Vec};

use crate::{pci::ata::{self, SECTOR_SIZE, SSECTOR_SIZE, Channel, Drive, get_disk}, dbg, serial_print_all_bits, serial_print, serial_println};

#[derive(Debug)]
pub enum DiskError {
    NotFound, 
    PermissionDenied, // Shouldn't happen... But keep this for rlib ?
    SectorTooBig,
    NoReadModeAvailable,
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
        let offset_from_sector_start = start%SECTOR_SIZE;
        let sector_address = start/SECTOR_SIZE;
        
        let size = self.len+offset_from_sector_start as u64;
        let mut sector_count = (size/SECTOR_SIZE as u64).try_into()?;
        if sector_count == 0 {sector_count = 1}

        let disk = get_disk(Channel::Primary, Drive::Master).unwrap();
        let sectors = disk.read_sectors(sector_address.into(), sector_count)?;
        let raw = unite_sectors(sectors);
        
        let start64 = start as usize;
        let mut content = String::new();
        let slice = &raw[start64..start64+self.len as usize];
        for (i,w) in slice.iter().enumerate() {
            content.push(((w >> 8) as u8) as char); //Transforms the word into two bytes
            content.push(((w & 0xFF) as u8) as char);// Interpreted as chars
        }
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



