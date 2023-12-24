use alloc::{vec::{Vec, self}, string::{String, ToString}};
use hashbrown::HashMap;
use log::error;

use crate::{drivers::{disk::{ata::{read_from_disk, DiskLoc, Channel, Drive}, DiskError}, fs::{Dir83Format, fs::{FilePath, Fat32File, BiosParameterBlock, FatAttributes, cluster_to_sector}}}, serial_print, serial_println};

use super::fs::{FileSystemError, GenericFile, Fat32Element, FatEntry};
pub enum FatType {
    ExFat,
    Fat12,
    Fat16,
    Fat32,
}
#[derive(Default)]
pub struct FatInfo(
    pub BiosParameterBlock,
);
impl FatInfo {
    pub fn first_sector_of_cluster(&self) -> u64 {
        let first_data_sector = self.get_first_data_sector();
        let first_root_dir_sector = first_data_sector - self.get_root_dir_sectors();
        ((self.0.root_dir_first_cluster - 2) * self.0.sectors_per_cluster as u32) as u64 + first_data_sector
    }
    pub fn get_first_data_sector(&self) -> u64 {
        let total_sectors = self.get_total_sectors();
        let fat_size = self.get_fat_size();
        let root_dir_sectors = self.get_root_dir_sectors();
        self.first_fat_sector() as u64 + (self.0.fats as u64 * fat_size as u64) + root_dir_sectors
    }
    pub fn fat_type(&self) -> FatType {
        let total_clusters = self.get_total_clusters();
        if(total_clusters < 4085) {FatType::Fat12}
            else if(total_clusters < 65525){FatType::Fat16}
            else {FatType::Fat32}
    }
    pub fn get_total_clusters(&self) -> u64 {
        self.get_data_sectors() as u64 / self.0.sectors_per_cluster as u64
    }
    pub fn get_data_sectors(&self) -> u64 {
        self.get_total_sectors() as u64 - (self.0.reserved_sectors as u64 + (self.0.fats as u64 * self.get_fat_size() as u64) + self.get_root_dir_sectors()) as u64
    }
    pub fn get_total_sectors(&self) -> u32 {
        if (self.0.total_sectors_16 == 0) {self.0.total_sectors_32} else {self.0.total_sectors_16.into()}
    }
    pub fn get_fat_size(&self) -> u32 {
        if (self.0.sectors_per_fat_16==0) {self.0.sectors_per_fat_32} else {self.0.sectors_per_fat_16 as u32}
    }
    pub fn get_root_dir_sectors(&self) -> u64 {
        ((self.0.root_entries as u64 * 32 as u64) + (self.0.bytes_per_sector as u64 - 1)) / self.0.bytes_per_sector as u64
    }
    pub fn first_fat_sector(&self) -> u16 {
        self.0.reserved_sectors
    }
}

pub type Files =   HashMap<FilePath, Fat32File>;
pub type Entries = HashMap<FilePath, FatEntry>;

pub struct FsDriver {
    opened_files: Files,
    entries: Entries,
    initialised: bool,
    pub fat_info: FatInfo,
    disk: DiskLoc,
}
impl FsDriver {
    pub fn open_file(&mut self, file_path: &FilePath) -> Result<GenericFile, FileSystemError> {
        todo!()
    }
    pub fn close_file(&mut self, file: &mut Fat32File) -> Result<(), FileSystemError> {
        if (self.opened_files.get(file.path()).is_some()) {
            self.opened_files.remove(file.path());
            Ok(())
        } else {
            Err(FileSystemError::FileNotFound)
        }
    }
    pub fn ls(&self, path: &FilePath) -> &Files {
        &self.opened_files
    }
    pub fn read_file(&self, path: &FilePath) -> Option<String> {
        let file = self.entries.get(path)?;
        let mut i = 0;
        let mut chars = Vec::new();
        let mut done_reading_file = false;
        while done_reading_file==false {
            for byte in read_from_disk(self.disk, file.sector+i, 1).unwrap_or(alloc::vec![0]) {
                if byte == 0 {
                    done_reading_file = true;
                    break
                }
                chars.push(byte);
            }
            i+=1;
        }
        Some(String::from_utf8_lossy(chars.as_slice()).to_string())
    }
    pub fn read_dir_at_sector(&self, dir_sector: u64) -> Option<Entries> {
        let mut entries = Entries::new();
        let mut reading = true;
        let mut next_sector = dir_sector;
        while reading {
            let sector = read_from_disk(self.disk, next_sector, 1).unwrap_or(alloc::vec![0]);
            if *sector.last().unwrap()==0 {
                let fat_offset = ((next_sector-self.fat_info.get_first_data_sector())+2)*4;
                let fat_sector = (fat_offset/512)+self.fat_info.first_fat_sector() as u64;
                let ent_offset = (fat_offset%512) as usize;
                let content = read_from_disk(self.disk, fat_sector, 1).unwrap();
                let table_value = &content[ent_offset..ent_offset+4];
                let mut table_value = 
                ((table_value[3] as u32) << 24)
                    | ((table_value[2] as u32) << 16)
                    | ((table_value[1] as u32) << 8)
                    | (table_value[0] as u32);
                table_value &= 0x0FFFFFFF;
                // serial_print!("Following cluster {:#x}", table_value);
                if table_value >= 0x0FFFFFF8 {
                    // serial_println!(" - no more clusters in chain.");
                    reading=false;
                } else if table_value == 0x0FFFFFF7 {
                    // serial_println!(" - bad cluster.");
                    reading=false;
                } else {
                    next_sector = cluster_to_sector(table_value as u64, self.fat_info.get_first_data_sector())-1; // -1 because we add one at end
                    // serial_println!(" -> {}", next_sector);
                }
            }

            let raw_entries_part = FsDriver::get_raw_entries(&sector);
            let entries_part = FsDriver::parse_entries(&raw_entries_part, self.fat_info.get_first_data_sector());
            entries.extend(entries_part);
            next_sector+=1;
        }
        Some(entries)
    }
    pub fn read_dir(&self, path: &FilePath) -> Option<Entries> {
        let p = self.entries.get(&FilePath::new(path.root().to_string()));
        let mut previous_dir = p?.clone();
        let mut previous_entries = self.read_dir_at_sector(previous_dir.sector).unwrap();
        let splitted: Vec<&str> = path.splitted().collect();
        if splitted.len()>1 {
            for dir in &splitted.as_slice()[1..] {
                previous_dir = previous_entries.get(&FilePath::new(dir.to_string()))?.clone();
                serial_println!("Reqding {:?}", previous_dir);
                previous_entries = self.read_dir_at_sector(previous_dir.sector).unwrap();
                serial_println!("{:?}", &previous_entries);
            }
            previous_entries = self.read_dir_at_sector(previous_dir.sector).unwrap();
        }
        Some(previous_entries)
    }
    pub fn new(disk: DiskLoc) -> Self {
        let mut entries = HashMap::new();
        let fat_info = FsDriver::get_fat_info(disk).unwrap();
        serial_println!("Fat info: {:?}", fat_info.0);
        for (path, file) in Self::get_entries_in_root(&fat_info, DiskLoc(Channel::Primary, Drive::Slave)).unwrap() {
            if file.is_file {serial_print!("FILE ")}
            else            {serial_print!("DIR  ")}
            serial_println!("{} {:?}", path.raw_path, file.sector);
            entries.insert(path, file);
        }
        // FsDriver::read_fat(disk, fat_info.0.reserved_sectors, fat_info.get_fat_size());
        let _self = Self {
            opened_files: HashMap::new(),
            entries,
            fat_info,
            initialised: false,
            disk,
        };
        _self
    }
    fn get_fat_boot(disk: DiskLoc) -> Result<BiosParameterBlock, DiskError> {
        let raw_fat_boot = read_from_disk(disk, 0, 2)?;
        let fat_boot = unsafe { &*(raw_fat_boot.as_ptr() as *const BiosParameterBlock) };
        Ok(fat_boot.clone())
    }
    fn get_fat_info(disk: DiskLoc) -> Result<FatInfo, DiskError> {
        let fat_boot = FsDriver::get_fat_boot(disk)?;
        
        let fat_info = FatInfo(fat_boot);
        Ok(fat_info)
    }
    fn get_raw_entries(sector: &Vec<u8>) -> Vec<&[u8]> {
        let mut entries = Vec::new();
        for i in 0..sector.len()/32 {
            let sector_section = &sector[(i*32)..(i*32)+31];

            if (sector_section[0]!=0xE5) {
                entries.push((sector_section));
            } else if sector_section[0]==0 {
                break
            }
        }
        entries
    }
    pub fn get_entries_in_root(fat_info: &FatInfo, disk: DiskLoc) -> Result<Entries, DiskError> {
        let first_sector_of_cluster = fat_info.first_sector_of_cluster();
        let sector = read_from_disk(disk, first_sector_of_cluster, 3).unwrap();
        let raw_entries = FsDriver::get_raw_entries(&sector);
        Ok(FsDriver::parse_entries(&raw_entries, fat_info.get_first_data_sector()))
    }

    fn parse_entries(raw_entries: &Vec<&[u8]>, first_data_sector: u64) -> Entries {
        let mut entries = Entries::new();
        for (i,raw) in raw_entries.iter().enumerate() {
            let entry = unsafe { &*(raw.as_ptr() as *const Dir83Format) };
            // serial_println!("{}", entry.printable(first_data_sector));
            if entry.attributes&0xF==0xF{ // Long File Name LFN
                let entry_name = Dir83Format::lfn_name(*raw);
                let next_entry = unsafe { &*(raw_entries[i+1].as_ptr() as *const Dir83Format) };
                // if next_entry.attributes&0x10==0x10 { // Directory
                let next_entry_name = next_entry.name();
                if next_entry_name.starts_with(".") || next_entry_name.starts_with("..") {continue} // Pass on "." and ".." folders
                
                let fst_cluster = ((next_entry.high_u16_1st_cluster as u32) << 16) | (next_entry.low_u16_1st_cluster as u32);
                let sector = (fst_cluster as u64 - 2)+first_data_sector;
                let is_file = next_entry.attributes&0x20==0x20;
                entries.insert(FilePath::new(entry_name), FatEntry::new(sector, is_file));
                // }
            }
            else if entry.attributes&0x08==0x08{
                serial_println!("VOLUME {} ", entry.printable(first_data_sector));
            }
            else if entry.attributes&0x01==0x01{
                serial_println!("READ_ONLY {} ", entry.printable(first_data_sector));
            }
            else if entry.attributes&0x02==0x02{
                serial_println!("HIDDEN {} ", entry.printable(first_data_sector));
            }
            else if entry.attributes&0x04==0x04{
                serial_println!("SYSTEM {} ", entry.printable(first_data_sector));
            } else if entry.attributes&0x10==0x10||entry.attributes&0x20==0x20{} // Dir 
            else if entry.attributes==0{}
            else {
                serial_println!("ELSE ({:#b}) {}", entry.attributes, entry.printable(first_data_sector));
            }
            // else if entry.attributes&0x20==0x20{    
            //     let fst_cluster = ((entry.high_u16_1st_cluster as u32) << 16) | (entry.low_u16_1st_cluster as u32);
            //     let sector = (fst_cluster as u64 - 2)+first_data_sector;
            //     // print_file(first_data_sector, entry);
            //     // serial_println!();
            //     serial_print!("FILE at {} {:#x}", sector, entry.name[0]);
            //     if entry.name[0]&0x40==0x40 {
            //         let size = entry.size;
            //         serial_println!(" {}Ko", size/1024);
            //     }
            // }
            // else if entry.attributes&0x10==0x10{
            //     let fst_cluster = ((entry.high_u16_1st_cluster as u32) << 16) | (entry.low_u16_1st_cluster as u32);
            //     let entry_name = entry.name();

            //     if entry_name.starts_with(".") || entry_name.starts_with("..") {
            //         serial_println!("Dir (symbolic) {}", entry_name);
            //         continue
            //     }
            //     if fst_cluster<2 {
            //         serial_print!("Invalid file with 0 fst cluster !");
            //         serial_println!("{}",entry.printable(first_data_sector));
            //         continue
            //     }
            //     let sector = (fst_cluster as u64 - 2)+first_data_sector;
            
            //     serial_println!("DIR at {}", sector);
            // }
        }
        entries
    }
}