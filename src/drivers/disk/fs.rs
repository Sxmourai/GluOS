use core::num::TryFromIntError;

use alloc::{string::{String, ToString, ParseError}, vec::Vec};

use crate::{serial_print_all_bits, serial_print, serial_println, u16_to_u8};

use super::{ata::{read_from_disk, SSECTOR_SIZE, SSECTOR_SIZEWORD, Sectors}, DiskError};


pub fn parse_sectors(sectors: &Sectors) -> String {
    let mut content = String::new();
    for sector in sectors {
        for b in sector {
            content.push(*b as char);
        }
    }//String::from_utf16_lossy(&sector).as_str()
    content
}

pub fn unite_sectors(sectors: Sectors) -> Vec<u8> {
    let mut united = Vec::new();
    for sector in sectors {
        for byte in sector {
            united.push(byte);
        }
    }
    united
}





// pub struct File {
//     name: String,
//     len: u64,
// }
// impl File {
//     pub fn new(name:String) -> Self {
//         Self {
//             name,
//             len: 10,
//         }
//     }
//     pub fn read(&self) -> Result<String, DiskError> {
//         let start = 0;
//         let sectors = read_from_disk(1u8, start, start+self.len)?;
//         let content = parse_sectors(&sectors);
//         Ok(content)
//     }
//     pub fn write(&self, content: String) -> Result<(), DiskError> {
//         Ok(())
//     }
//     pub fn delete(&self) -> Result<(), DiskError> {
//         Ok(())
//     }
// }
// // create [--object objectdef] [-q] [-f fmt] [-b backing_file] [-F backing_fmt] [-u] [-o options] filename [size]
// pub fn open(filename: &str) -> Result<File, DiskError> {
//     Ok(File::new(filename.to_string()))
// }
// pub fn read(filename: &str) -> Result<String, DiskError> {
//     open(filename)?.read()
// }
// pub fn write(filename: &str, content: &str) -> Result<(), DiskError> {
//     open(filename)?.write(content.to_string())
// }
// pub fn delete(filename: &str) -> Result<(), DiskError> {
//     open(filename)?.delete()
// }