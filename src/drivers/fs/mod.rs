use core::{cell::Cell, fmt::{Display, Debug}};

use crate::{state::get_state, serial_println, serial_print};

use self::fs::{FilePath, FileSystemError, GenericFile, BiosParameterBlock};

pub mod fs_driver;
pub mod fs;

use alloc::{boxed::Box, vec::{Vec, self}, string::{String, ToString}, format};
use hashbrown::HashMap;
use log::error;
use spin::{Mutex, MutexGuard};

use super::disk::{ata::{DiskLoc, read_from_disk}, DiskError};
#[repr(packed)]
pub struct Dir83Format {
    // r:u8,
    name: [u8; 11],
    attributes: u8,
    reserved: u8,
    duration_creation_time: u8,
    creation_time: u16,
    creation_date: u16,
    last_accessed_date: u16,
    high_u16_1st_cluster: u16,
    last_modif_time: u16,
    last_modif_date: u16,
    low_u16_1st_cluster: u16,
    size: u32
}
impl Dir83Format {
    pub fn lfn_name(raw_self: &[u8]) -> String {
        let mut name = String::new();
        let mut raw_name = raw_self[1..11].to_vec();
        raw_name.extend_from_slice(&raw_self[14..=26]);
        raw_name.extend_from_slice(&raw_self[28..31]);
        for chunk in raw_name.chunks_exact(2) {
            let chr = u16::from_ne_bytes([chunk[0], chunk[1]]);
            if chr == 0 {break}
            name.push_str(String::from_utf16_lossy(&[chr]).as_str());
        }
        name
    }
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }
    pub fn printable(&self, first_data_sector:u64) -> String {
        let creation_date = self.creation_date;
        let fst_cluster = ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let sector  = if fst_cluster>2 {
            (fst_cluster as u64 - 2)+first_data_sector
        } else { 0 };
        let size = self.size;
        let entry_type = self.name[0];
        let name = self.name();
        format!("File8.3: {}\t | creation_date: {} | 1st cluster: {}({}) \t| size: {}", name, creation_date, fst_cluster,sector, size)
    }
}
