use alloc::{vec::{Vec, self}, string::{String, ToString}, boxed::Box, format};
use hashbrown::HashMap;
use log::{error, debug};


use crate::{disk::{ata::{DiskLoc, read_from_disk, Channel, Drive}, DiskError}, serial_println, serial_print};
use super::{fs::*, userland::FatAttributes};
use super::entry::Dir83Format;

pub type Files =   HashMap<FilePath, Fat32Entry>;

pub struct FsDriver {
    files: Files,
    initialised: bool,
    pub fat_info: FatInfo,
    disk: DiskLoc,
}
impl FsDriver {
    // pub fn open_file(&mut self, file_path: &FilePath) -> Result<Fat32File, FileSystemError> {
    //     todo!()
    // }
    // pub fn close_file(&mut self, file: &mut Fat32File) -> Result<(), FileSystemError> {
    //     if (self.opened_files.get(file.path()).is_some()) {
    //         self.opened_files.remove(file.path());
    //         Ok(())
    //     } else {
    //         Err(FileSystemError::FileNotFound)
    //     }
    // }
    pub fn read_file(&self, path: &FilePath) -> Option<String> {
        let file = match self.files.get(path)? {
            Fat32Entry::File(file) => file,
            Fat32Entry::Dir(_) => return None,
        };
        let mut i = 0;
        let mut chars = Vec::new();
        let mut reading = true;
        while reading {
            for byte in read_from_disk(&self.disk, file.sector() as u64+i, 1).unwrap_or(alloc::vec![0]) {
                if byte == 0 {
                    reading = false;
                    break
                }
                chars.push(byte);
            }
            i+=1;
        }
        Some(String::from_utf8_lossy(chars.as_slice()).to_string())
    }
    pub fn read_dir_at_sector(&self, prefix: &FilePath, dir_sector: u64) -> Option<Files> {
        let mut entries = Files::new();
        let mut reading = true;
        let mut next_sector = dir_sector;
        while reading {
            let sector = read_from_disk(&self.disk, next_sector, 1).unwrap_or(alloc::vec![0]);
            if *sector.last().unwrap()==0 {
                let fat_offset = ((next_sector-self.fat_info.get_first_data_sector())+2)*4;
                let fat_sector = (fat_offset/512)+self.fat_info.first_fat_sector() as u64;
                let ent_offset = (fat_offset%512) as usize;
                let content = read_from_disk(&self.disk, fat_sector, 1).unwrap();
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

            let raw_entries_part = Self::get_raw_entries(&sector);
            let entries_part = Self::parse_entries(&raw_entries_part, self.fat_info.get_first_data_sector(), &prefix);
            entries.extend(entries_part);
            next_sector+=1;
        }
        Some(entries)
    }
    pub fn get_entry(&self, path: &FilePath) -> Option<&Fat32Entry> {
        self.files.get(path)
    }
    pub fn read_dir(&self, path: &FilePath) -> Option<Files> {
        let dir = match self.files.get(path)? {
            Fat32Entry::File(_) => return None,
            Fat32Entry::Dir(dir) => dir,
        };
        self.read_dir_at_sector(path, dir.sector as u64)
    }
    pub fn new(disk: DiskLoc) -> Self {
        let fat_info = Self::get_fat_info(&disk).unwrap();
        let _self = Self {
            files: Self::read_dirs_structure(&fat_info, &disk).unwrap(),
            fat_info,
            initialised: false,
            disk,
        };
        // serial_println!("{:#?}", _self.files);
        _self
    }
    fn get_fat_boot(disk: &DiskLoc) -> Result<BiosParameterBlock, DiskError> {
        let raw_fat_boot = read_from_disk(disk, 0, 2)?;
        let fat_boot = unsafe { &*(raw_fat_boot.as_ptr() as *const BiosParameterBlock) };
        Ok(fat_boot.clone())
    }
    fn get_fat_info(disk: &DiskLoc) -> Result<FatInfo, DiskError> {
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
    pub fn read_dirs_structure(fat_info: &FatInfo, disk: &DiskLoc) -> Result<Files, DiskError> {
        let mut sector = fat_info.first_sector_of_cluster();
        let first_data_sector = fat_info.get_first_data_sector();
        let mut files = Self::read_dir_recursively("/".to_filepath(), disk, sector, first_data_sector);
        let root = FilePath::new("/".to_string());
        files.insert(root.clone(), Fat32Entry::Dir(Fat32Dir {
            path: root,
            attributes: FatAttributes::default(),
            sector: sector as u32,
            
        }));
        Ok(files)
    }
    fn read_dir_recursively(prefix: FilePath, disk: &DiskLoc, sector: u64, first_data_sector: u64) -> Files {
        let raw_sector = read_from_disk(disk, sector, 1).unwrap();
        let mut files = Files::new();
        let raw_entries = Self::get_raw_entries(&raw_sector);
        for (path, entry) in Self::parse_entries(&raw_entries, first_data_sector, &prefix) {
            let entry = match entry {
                Fat32Entry::File(mut file) => {
                    Fat32Entry::File(file)
                },
                Fat32Entry::Dir(mut dir) => {
                    let dir2 = dir.clone();
                    let dir_name = dir.name().to_filepath();
                    let inner_entries = Self::read_dir_recursively(prefix.clone().join(dir_name.clone()), disk, dir.sector() as u64, first_data_sector);
                    files.extend(inner_entries);
                    Fat32Entry::Dir(dir2)
                },
            };
            files.insert(path, entry);
        }
        files
    }
    fn parse_entries(raw_entries: &Vec<&[u8]>, first_data_sector: u64, prefix: &FilePath) -> Files {
        let mut files = Files::new();
        for (i,raw) in raw_entries.iter().enumerate() {
            let entry = unsafe { &*(raw.as_ptr() as *const Dir83Format) };
            if entry.attributes&0xF==0xF{ // Long File Name LFN
                let entry_name = Dir83Format::lfn_name(*raw);
                let next_entry = unsafe { &*(raw_entries[i+1].as_ptr() as *const Dir83Format) };
                // if next_entry.attributes&0x10==0x10 { // Directory
                let next_entry_name = next_entry.name();
                if next_entry_name.starts_with(".") || next_entry_name.starts_with("..") {continue} // Pass on "." and ".." folders
                
                let fst_cluster = ((next_entry.high_u16_1st_cluster as u32) << 16) | (next_entry.low_u16_1st_cluster as u32);
                let sector = (fst_cluster - 2)+first_data_sector as u32;
                let is_file = next_entry.attributes&0x20==0x20;
                let path = prefix.clone().join(entry_name.to_filepath());
                let p2=path.clone();
                let parsed_entry = if is_file { Fat32Entry::File(Fat32File {
                        size: entry.size as u64,
                        path,
                        attributes: FatAttributes::default(),
                        sector,
                    })
                } else { Fat32Entry::Dir(Fat32Dir {
                        path,
                        attributes: FatAttributes::default(),
                        sector,
                    })
                };
                files.insert(p2, parsed_entry);
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
        }
        files
    }
}