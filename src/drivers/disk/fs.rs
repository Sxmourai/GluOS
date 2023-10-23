use core::num::TryFromIntError;

use alloc::{string::{String, ToString, ParseError}, vec::Vec};

use crate::{serial_print_all_bits, serial_print, serial_println, u16_to_u8};

use super::{ata::{read_from_disk, SSECTOR_SIZE, SSECTOR_SIZEWORD}, DiskError};


pub fn parse_sectors(sectors: &Vec<[u16; SSECTOR_SIZEWORD]>) -> String {
    let mut content = String::new();
    for sector in sectors {
        for w in sector {
            let (l1, l2) = u16_to_u8(*w);
            content.push(l2 as char);
            content.push(l1 as char);
        }
    }//String::from_utf16_lossy(&sector).as_str()
    content
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
        let sectors = read_from_disk(1u8, start, start+self.len)?;
        let content = parse_sectors(&sectors);
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

pub fn unite_sectors(sectors: Vec<[u16;SSECTOR_SIZE/2]>) -> Vec<u16> {
    let mut united = Vec::new();
    for sector in sectors {
        for word in sector {
            united.push(word);
        }
    }
    united
}



