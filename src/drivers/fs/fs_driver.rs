use alloc::{vec::{Vec, self}, string::{String, ToString}, boxed::Box, format};
use hashbrown::HashMap;
use log::{error, debug, info};


use crate::{disk::{ata::{DiskLoc, read_from_disk, Channel, Drive, write_to_disk}, DiskError}, serial_println, serial_print, bit_manipulation::any_as_u8_slice};
use super::{fs::*, userland::FatAttributes};
use super::entry::Dir83Format;

pub type Files =   HashMap<FilePath, Fat32Entry>;

pub struct FsDriver {
    files: Files,
    initialised: bool,
    pub fat_info: FatInfo,
    disk: DiskLoc,
    fat_table: FatTable,
}
impl FsDriver {
    pub fn new(disk: DiskLoc) -> Self {
        let fat_info = Self::get_fat_info(&disk).unwrap();
        let first_fat_sector = fat_info.first_fat_sector();
        let first_data_sector = fat_info.get_first_data_sector();
        // serial_println!("{:?} {} {}", &fat_info, fat_info.first_fat_sector(), fat_info.get_first_data_sector());
        let (mut last_sector, mut last_offset)=(0,0);
        let mut last_meaningful =0;
        serial_println!("Reading fat at {}", fat_info.first_fat_sector());
        for i in 0..fat_info.get_fat_size() {
            if last_sector!=0{break}
            let content = read_from_disk(&disk, first_fat_sector as u64+i as u64, 1).unwrap();
            for offset in 0..content.len()/4 {
                let table_value = &content[offset*4..offset*4+4];
                let mut table_value = 
                ((table_value[3] as u32) << 24)
                | ((table_value[2] as u32) << 16)
                | ((table_value[1] as u32) << 8)
                | ( table_value[0] as u32);
                table_value &= 0x0FFFFFFF;
                if table_value < 0x0FFFFFF8 && table_value != 0x0FFFFFF7 {
                    if table_value==0 {
                        last_sector = first_fat_sector+i as u16;
                        last_offset = offset as u16;
                        let table_value = &content[(offset-1)*4..(offset-1)*4+4];
                        let mut table_value = 
                        ((table_value[3] as u32) << 24)
                        | ((table_value[2] as u32) << 16)
                        | ((table_value[1] as u32) << 8)
                        | ( table_value[0] as u32);
                        table_value &= 0x0FFFFFFF;
                        serial_println!("Last sector used: {}", last_meaningful);
                        break;
                    }
                    let next_sector = cluster_to_sector(table_value as u64, first_data_sector)-1; // -1 because we add one at end
                    last_meaningful=next_sector;
                }
            }
        }
        let fat_size = fat_info.get_fat_size();
        let _self = Self {
            files: Self::read_dirs_structure(&fat_info, &disk).unwrap(),
            fat_info,
            initialised: false,
            disk,
            fat_table: FatTable {
                size: fat_size,
                first_sector: first_fat_sector,
                last_sector,
                last_offset,
            },
        };
        // serial_println!("{:?}", (last_sector, last_offset));
        _self
    }
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
    pub fn read_dir(&self, path: &FilePath) -> Option<Files> {
        let dir = match self.files.get(path)? {
            Fat32Entry::File(_) => return None,
            Fat32Entry::Dir(dir) => dir,
        };
        self.read_dir_at_sector(path, dir.sector as u64)
    }
    pub fn write_file(&mut self, path: impl Into<FilePath>, content: String) -> Result<(), FileSystemError> {
        todo!()
    }
    pub fn write_dir(&mut self, path: impl Into<FilePath>) -> Result<(), FileSystemError> {
        let path = path.into();
        serial_println!("Files: {:?} {:?}", self.files, path.parent());
        if let Some(entry) = self.files.get(&Into::<FilePath>::into(path.parent())) {
            let start_sector = match entry {
                Fat32Entry::File(_) => todo!(),
                Fat32Entry::Dir(dir) => dir.sector,
            };
            let content = unsafe { any_as_u8_slice(entry) };
            serial_println!("Writing {:?} - {:?}", content, String::from_utf8_lossy(content));
            let mut bytes = Vec::new();
            for c in content {
                bytes.push(*c);
            }
            write_to_disk(self.disk, start_sector as u64, bytes);
            let sector = 0;
            self.files.insert(path.clone(), Fat32Entry::Dir(Fat32Dir { 
                path,
                attributes: FatAttributes::default(),
                sector
            }));
        } else {
            error!("whilst trying to write dir in {} (parent: {})", path, path.parent());
        }
        Ok(())
    }
    pub fn read_dir_at_sector(&self, prefix: &FilePath, dir_sector: u64) -> Option<Files> {
        let mut entries = Files::new();
        let mut reading = true;
        let mut next_sector = dir_sector;
        let first_data_sector = self.fat_info.get_first_data_sector();
        let first_fat_sector = self.fat_info.first_fat_sector() as u64;
        while reading {
            let sector = read_from_disk(&self.disk, next_sector, 1).unwrap_or(alloc::vec![0]);
            if *sector.last().unwrap()==0 {
                let next_cluster = match self.read_fat_cluster(next_sector, first_fat_sector, first_data_sector) {
                    ClusterEnum::EndOfChain => {reading=false;break},
                    ClusterEnum::BadCluster => {reading=false;break},
                    ClusterEnum::Cluster(cluster) => cluster,
                };
                next_sector = cluster_to_sector(next_cluster as u64, first_data_sector)-1 // -1 Because we add one below
                
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
    // Current sector should be u32
    pub fn read_fat_cluster(&self, current_sector: u64, first_fat_sector: u64, first_data_sector: u64) -> ClusterEnum {
        let fat_offset = ((current_sector-first_data_sector)+2)*4;
        let fat_sector = (fat_offset/512)+first_fat_sector;
        let ent_offset = (fat_offset%512) as usize;
        let content = read_from_disk(&self.disk, fat_sector, 1).unwrap();
        let table_value = &content[ent_offset..ent_offset+4];
        let mut table_value = 
        ((table_value[3] as u32) << 24)
            | ((table_value[2] as u32) << 16)
            | ((table_value[1] as u32) << 8)
            | (table_value[0] as u32);
        table_value &= 0x0FFFFFFF;
        if table_value >= 0x0FFFFFF8 {
            ClusterEnum::EndOfChain
        } else if table_value == 0x0FFFFFF7 {
            ClusterEnum::BadCluster
        } else {
            ClusterEnum::Cluster(table_value)
        }
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
        let mut files = Self::read_dir_recursively("/".into(), disk, sector, first_data_sector);
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
        serial_println!("Raw Entries: at {}: {:?}", sector, raw_entries);
        for (path, entry) in Self::parse_entries(&raw_entries, first_data_sector, &prefix) {
            serial_println!("Entry: {:?}", entry);
            let entry = match entry {
                Fat32Entry::File(mut file) => {
                Fat32Entry::File(file)
                },
                Fat32Entry::Dir(mut dir) => {
                    let dir2 = dir.clone();
                    let dir_name = Into::<FilePath>::into(dir.name());
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
                if i+1>=raw_entries.len() {break} // Continue or break ? It's last element
                let next_entry = unsafe { &*(raw_entries[i+1].as_ptr() as *const Dir83Format) };
                // if next_entry.attributes&0x10==0x10 { // Directory
                let next_entry_name = next_entry.name();
                if next_entry_name.starts_with(".") || next_entry_name.starts_with("..") {continue} // Pass on "." and ".." folders
                
                let fst_cluster = ((next_entry.high_u16_1st_cluster as u32) << 16) | (next_entry.low_u16_1st_cluster as u32);
                if fst_cluster>1_000_000 {
                    serial_println!("Cluster to big ! {} {}", fst_cluster, next_entry.printable(first_data_sector));
                    break
                }
                let sector = (fst_cluster - 2)+first_data_sector as u32;
                let is_file = next_entry.attributes&0x20==0x20;
                let path = prefix.clone().join(entry_name.into());
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
            } else if entry.attributes&0x10==0x10 || entry.attributes&0x20==0x20{} // Dir 
            else if entry.attributes==0{}
            else {
                serial_println!("ELSE ({:#b}) {}", entry.attributes, entry.printable(first_data_sector));
            }
        }
        files
    }
}