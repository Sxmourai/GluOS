use core::num::TryFromIntError;

use alloc::{string::{String, ToString, ParseError}, vec::Vec};
use fatfs::IntoStorage;
use log::error;

use crate::{serial_print_all_bits, serial_print, serial_println, u16_to_u8, println, print};

use super::{ata::{read_from_disk, SSECTOR_SIZE, SSECTOR_SIZEWORD, Sectors, DiskLoc, Disk, disk_manager}, DiskError};


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


pub fn read_fat_disk(loc: DiskLoc) {
    if let Ok(raw_fat_boot) = read_from_disk(loc, 0, 2) {
        let fat_boot = unsafe { &*(raw_fat_boot.as_ptr() as *const BiosParameterBlock) };
        let total_sectors = if (fat_boot.total_sectors_16 == 0) {fat_boot.total_sectors_32} else {fat_boot.total_sectors_16.into()};
        let fat_size = if (fat_boot.sectors_per_fat_16==0) {fat_boot.sectors_per_fat_32} else {fat_boot.sectors_per_fat_16 as u32};
        let res = fat_boot.bytes_per_sector;
        println!("{:?}",fat_boot);
        let root_dir_sectors = ((fat_boot.root_entries as u64 * 32 as u64) + (fat_boot.bytes_per_sector as u64 - 1)) / fat_boot.bytes_per_sector as u64;
        let first_data_sector = fat_boot.reserved_sectors as u64 + (fat_boot.fats as u64 * fat_size as u64) + root_dir_sectors;
        let first_fat_sector = fat_boot.reserved_sectors;
        let data_sectors = total_sectors as u64 - (fat_boot.reserved_sectors as u64 + (fat_boot.fats as u64 * fat_size as u64) + root_dir_sectors) as u64;
        let total_clusters = data_sectors as u64 / fat_boot.sectors_per_cluster as u64;
        let fat_type = 
            // if (sectorsize == 0) {"ExFAT"}
            if(total_clusters < 4085) {"FAT12"}
            else if(total_clusters < 65525){"FAT16"}
            else {"FAT32"};
        println!("total_sectors: {}\nfat_size: {}\nroot_dir_sectors: {}\nfirst_data_sector: {}\nfirst_fat_sector: {}\ndata_sectors: {}\ntotal_clusters: {}\n\nfat_type: {}",
        total_sectors,fat_size,root_dir_sectors,first_data_sector,first_fat_sector,data_sectors,total_clusters,fat_type);
        let first_root_dir_sector = first_data_sector - root_dir_sectors;
        let root_cluster_32 = fat_boot.root_dir_first_cluster;
        let first_sector_of_cluster = ((root_cluster_32 - 2) * fat_boot.sectors_per_cluster as u32) as u64 + first_data_sector;
        let sector = read_from_disk(loc, first_sector_of_cluster, 3).unwrap();
        for i in 0..sector.len()/32 {
            let base = (i*32);
            if (sector[base+0]==0) {
                // println!("Dir is empty")
            } else if (sector[base+0]==0xE5) {
                // println!("Dir unused")
            } else if (sector[base+11]==0x0F) {
                let mut name = String::new();
                for (i,b) in sector[base+1..base+11].iter().enumerate().step_by(2) {
                    name.push_str(&String::from_utf16_lossy( &[(sector[base+1..base+11][i+1] as u16) << 8 |  sector[base+1..base+11][i] as u16]));
                }
                println!("{name}")
        }
        }
    } else {
        error!("Error reading disk to read bios parameter block")
    }
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