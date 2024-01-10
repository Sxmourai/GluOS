use alloc::{vec::{Vec, self}, string::{String, ToString}, boxed::Box, format};
use hashbrown::HashMap;
use log::{error, debug, info};


use crate::{disk::{ata::{DiskLoc, read_from_disk, Channel, Drive, write_to_disk}, DiskError}, serial_println, serial_print, bit_manipulation::any_as_u8_slice, dbg, fs::entry::Standard32};
use super::{fs::*, userland::FatAttributes, entry::{RawFat32Entry, LFN32}};


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
        let fat_table = FsDriver::read_fat(&disk, fat_info.get_fat_size(), first_fat_sector, first_data_sector);
        let files = Self::read_dirs_structure(&fat_info, &disk).unwrap();
        let _self = Self {
            files,
            fat_info,
            initialised: false,
            disk,
            fat_table,
        };
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
        if let Some(parent_entry) = self.files.get(&Into::<FilePath>::into(path.parent())) {
            let start_sector = match parent_entry {
                Fat32Entry::File(_) => todo!(),
                Fat32Entry::Dir(dir) => dir.sector,
            };
            dbg!(start_sector);
            let path = Into::<FilePath>::into(path);
            let mut path_name: Vec<u8> = path.name().bytes().collect();
            if path_name.len() < 11 { // Add some 0 to convert to [u8; 11]
                for i in 0..11-path_name.len() {
                    path_name.push(0);
                }
            }
            let name = <[u8; 8]>::try_from(path_name.clone()).unwrap();
            let extension = <[u8; 3]>::try_from(path_name[8..].to_vec()).unwrap();
            let size = 1;
            let to_write_sector = self.fat_table.last_sector+1;
            dbg!(to_write_sector);
            dbg!(self.fat_info.get_first_data_sector());
            let cluster = sector_to_cluster(to_write_sector as u64, self.fat_info.get_first_data_sector());
            let entry = Standard32 {
                name,
                extension,
                high_u16_1st_cluster: TryInto::<u16>::try_into(cluster & 0xFF00).unwrap(),
                low_u16_1st_cluster:  TryInto::<u16>::try_into(cluster & 0x00FF).unwrap(),
                size,
                ..core::default::Default::default()
            };
            let content = unsafe { any_as_u8_slice(&entry) };
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
    
    pub fn read_and_follow_clusters(disk: &DiskLoc, start_sector: u64, first_data_sector: u64, first_fat_sector: u64) -> Option<Vec<u8>> {
        let mut res = Vec::new();
        let mut reading = true;
        let mut next_sector = start_sector;
        while reading {
            let mut sector = read_from_disk(disk, next_sector, 1).unwrap_or(alloc::vec![0]);
            // if *sector.last().unwrap()==0 {
            if let Some(cluster) = Self::read_fat_cluster(disk, next_sector, first_fat_sector, first_data_sector) {
                match cluster {
                    ClusterEnum::EndOfChain => {reading=false},
                    ClusterEnum::BadCluster => {reading=false},
                    ClusterEnum::Cluster(cluster) => {
                        next_sector = cluster_to_sector(cluster as u64, first_data_sector);
                    },
                };
                
            } else {
                error!("Sector too small ?!");
                dbg!(Self::read_fat_cluster(disk, next_sector, first_fat_sector, first_data_sector))
            }

            res.append(&mut sector);
            // next_sector+=1;
        }
        Some(res)
    }

    pub fn read_dir_at_sector(&self, prefix: &FilePath, dir_sector: u64) -> Option<Files> {
        let mut entries = Files::new();
        let sectors = Self::read_and_follow_clusters(&self.disk, dir_sector, self.fat_info.get_first_data_sector(), self.fat_info.first_fat_sector() as u64)?;
        let raw_entries_part = Self::get_raw_entries(&sectors);
        let entries_part = Self::parse_entries(&raw_entries_part, self.fat_info.get_first_data_sector(), &prefix);
        Some(entries_part)
    }
    pub fn get_entry(&self, path: &FilePath) -> Option<&Fat32Entry> {
        self.files.get(path)
    }
    fn read_fat(disk: &DiskLoc,fat_size: u32, first_fat_sector: u16, first_data_sector: u64) -> FatTable {
        let (mut last_sector, mut last_offset) = (0,0);
        let mut last_used_sector = 0;
        for i in 0..fat_size {
            if last_sector!=0{break}
            let content = read_from_disk(disk, first_fat_sector as u64+i as u64, 1).unwrap();
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
                        break;
                    }
                    last_used_sector=cluster_to_sector(table_value as u64, first_data_sector) as u32;
                }
            }
        }
        FatTable {
            size: fat_size,
            first_sector: first_fat_sector,
            last_sector,last_offset,
            last_used_sector,
        }
    }
    // Current sector should be u32
    // Reads the fat table to know where is the next cluster to follow
    pub fn read_fat_cluster(disk: &DiskLoc, current_sector: u64, first_fat_sector: u64, first_data_sector: u64) -> Option<ClusterEnum> {
        if current_sector<first_data_sector { // Should do current_sector-2 but we could have buffer underflows...
            error!("Can't read cluster if sector is to small ! {}<{}", current_sector, first_data_sector);
            return None;
        }
        let fat_offset = ((current_sector-first_data_sector)+2)*4;
        let fat_sector = (fat_offset/512)+first_fat_sector;
        let ent_offset = (fat_offset%512) as usize;
        let content = read_from_disk(disk, fat_sector, 1).unwrap();
        let table_value = &content[ent_offset..ent_offset+4];
        let mut table_value = 
        ((table_value[3] as u32) << 24)
            | ((table_value[2] as u32) << 16)
            | ((table_value[1] as u32) << 8)
            | (table_value[0] as u32);
        table_value &= 0x0FFFFFFF;
        if table_value >= 0x0FFFFFF8 {
            Some(ClusterEnum::EndOfChain)
        } else if table_value == 0x0FFFFFF7 {
            Some(ClusterEnum::BadCluster)
        } else {
            Some(ClusterEnum::Cluster(table_value))
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
    
    pub fn read_dirs_structure(fat_info: &FatInfo, disk: &DiskLoc) -> Result<Files, DiskError> {
        let mut root_sector = fat_info.first_sector_of_cluster();
        let first_data_sector = fat_info.get_first_data_sector();
        let mut files = Self::read_dir_recursively("/".into(), disk, root_sector, first_data_sector, fat_info.first_fat_sector() as u64);
        let root = FilePath::new("/".to_string());
        files.insert(root.clone(), Fat32Entry::Dir(Fat32Dir {
            path: root,
            attributes: FatAttributes::default(),
            sector: root_sector as u32,
        }));
        Ok(files)
    }
    fn read_dir_recursively(prefix: FilePath, disk: &DiskLoc, sector: u64, first_data_sector: u64, first_fat_sector: u64) -> Files {
        let mut files = Files::new();
        let raw_sector = Self::read_and_follow_clusters(disk, sector, first_data_sector, first_fat_sector).unwrap();
        let raw_entries = Self::get_raw_entries(&raw_sector);
        for (path, entry) in Self::parse_entries(&raw_entries, first_data_sector, &prefix) {
            let entry = match entry {
                Fat32Entry::File(mut file) => {
                    Fat32Entry::File(file)
                },
                Fat32Entry::Dir(mut dir) => {
                    let dir2 = dir.clone();
                    let dir_name = Into::<FilePath>::into(dir.name());
                    let inner_entries = Self::read_dir_recursively(prefix.clone().join(dir_name.clone()), disk, dir.sector() as u64, first_data_sector, first_fat_sector);
                    files.extend(inner_entries);
                    Fat32Entry::Dir(dir2)
                },
            };
            files.insert(path, entry);
            }
        files
    }
    fn get_raw_entries(sector: &Vec<u8>) -> Vec<RawFat32Entry> {
        let mut entries = Vec::new();
        for i in 0..sector.len()/32 {
            let sector_section = &sector[(i*32)..(i*32)+31];
            if sector_section[0]==0 {
                break
            } else if sector_section[0]!=0xE5 {
                let attribute = sector_section[11];
                let entry = if attribute==0xF {
                    RawFat32Entry::LFN(unsafe {&*(sector_section.as_ptr() as *const LFN32)}.clone())
                } else {
                    RawFat32Entry::Standard(unsafe {&*(sector_section.as_ptr() as *const Standard32)}.clone())
                };
            // if entry.attributes&0x08==0x08{
            //     serial_println!("VOLUME {} ", entry.printable(first_data_sector));
            // }
            // else if entry.attributes&0x01==0x01{
            //     serial_println!("READ_ONLY {} ", entry.printable(first_data_sector));
            // }
            // else if entry.attributes&0x02==0x02{
            //     serial_println!("HIDDEN {} ", entry.printable(first_data_sector));
            // }
            // else if entry.attributes&0x04==0x04{
            //     serial_println!("SYSTEM {} ", entry.printable(first_data_sector));
            // } else if entry.attributes&0x10==0x10 || entry.attributes&0x20==0x20{} // Dir 
            // else if entry.attributes==0{}
            // else {
            //     serial_println!("ELSE ({:#b}) {}", entry.attributes, entry.printable(first_data_sector));
            // };
                entries.push(entry);
            }
        }
        entries
    }
    fn parse_entries(entries: &Vec<RawFat32Entry>, first_data_sector: u64, prefix: &FilePath) -> Files {
        let mut files = Files::new();
        let mut i = 0;
        while i < entries.len() {
            let entry = &entries[i];
            i+=1;
            match entry {
                RawFat32Entry::LFN(file) => {
                    let mut name = file.name();
                    let mut nentry = None;
                    loop {
                        if i >= entries.len() {
                            dbg!(file);
                            dbg!(entries[i-1]);
                            break
                        } else {
                            match &entries[i] {
                                RawFat32Entry::LFN(lfn) => {
                                    let mut new_name = lfn.name();
                                    new_name.push_str(&name);
                                    name = new_name;
                                    i+=1;
                                },
                                RawFat32Entry::Standard(file) => {nentry = Some(file); break},
                            };
                        }
                    }
                    if nentry.is_none() {continue}
                    let nentry = nentry.unwrap();
                    let next_name = nentry.name();
                    if next_name.starts_with(".") || next_name.starts_with("..") {continue} // Pass on "." and ".." folders
                    let is_file = nentry.attributes&0x20==0x20;
                    let path = prefix.clone().join(name.into());

                    let mut fst_cluster = ((nentry.high_u16_1st_cluster as u32) << 16) | (nentry.low_u16_1st_cluster as u32);
                    if fst_cluster>1_000_000 {
                        serial_print!("Cluster to big ! {} {}", fst_cluster, nentry.printable(first_data_sector));
                    }
                    let sector = if fst_cluster>2 { 
                        cluster_to_sector(fst_cluster as u64, first_data_sector) 
                    } else if fst_cluster==0 { // File is empty
                        0
                    } else {
                        error!("Cluster is too low ! ({})", fst_cluster);
                        dbg!(path);
                        i+=1;
                        continue
                    } as u32;

                    let parsed_entry = if is_file { Fat32Entry::File(Fat32File {
                            size: nentry.size as u64,
                            path: path.clone(),
                            attributes: FatAttributes::default(),
                            sector,
                        })
                    } else { Fat32Entry::Dir(Fat32Dir {
                            path: path.clone(),
                            attributes: FatAttributes::default(),
                            sector,
                        })
                    };
                    files.insert(path, parsed_entry);
                    
                    i+=1; // Skip next entry cuz it's related to the current LFN
                },
                RawFat32Entry::Standard(file) => { // From my tests only "." and ".." folders
                    if !file.name().starts_with(".") && !file.name().contains("CACHEDIRTAG") { //EDIT Also CACHEDIRTAG file
                        debug!("What is this file ?"); // If this prints one day we need to do investigations
                        dbg!(file)
                    }
                },
            }
        }
        files
    }
}