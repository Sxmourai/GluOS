use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec::Vec,
};
use hashbrown::HashMap;

use crate::{
    bit_manipulation::any_as_u8_slice,
    dbg,
    disk::{
        driver::{read_from_partition, write_to_partition},
        DiskError,
    },
    fs::path::FileSystemError,
    serial_print, serial_println,
};

use super::{
    fs_driver::{
        Dir, Entry, File, FsDriver, FsDriverEnum, FsDriverInitialiser, FsReadError, SoftEntry,
    },
    partition::Partition,
    path::FilePath,
    userland::FatAttributes,
};

#[derive(Debug)]
pub struct Fat32Driver {
    files: HashMap<FilePath, Fat32SoftEntry>,
    pub fat_info: FatInfo,
    pub partition: Partition,
    fat_table: FatTable,
}
impl Fat32Driver {
    pub fn new(partition: &Partition) -> Option<Self> {
        let fat_info = Self::get_fat_boot(partition).unwrap();
        if fat_info.0.fs_type_label[0..5] != [70, 65, 84, 51, 50] {
            // log::error!("Error reading fat info in {:?} {}",partition, crate::bit_manipulation::as_chars(&fat_info.0.fs_type_label));
            return None;
        }
        let first_fat_sector = fat_info.first_fat_sector();
        let first_data_sector = fat_info.get_first_data_sector();
        let fat_table = Self::read_fat(
            partition,
            fat_info.get_fat_size(),
            first_fat_sector,
            first_data_sector,
        );
        let root = FilePath::new("/".to_string(), partition.clone());
        let root_sector = fat_info.first_sector_of_cluster();
        let mut files = Self::walk_dir(
            root.clone(),
            partition,
            root_sector,
            first_data_sector,
            fat_info.first_fat_sector() as u64,
        );
        files.insert(
            root.clone(),
            Fat32SoftEntry {
                path: root,
                is_file: false,
                sector: root_sector,
            },
        );
        Some(Self {
            files,
            fat_info,
            partition: partition.clone(),
            fat_table,
        })
    }
    pub fn get_sector(&self, path: &FilePath) -> Option<u64> {
        Some(self.files.get(path)?.sector)
    }
    pub fn read_file(&self, path: &FilePath) -> Option<String> {
        let sector = self.get_sector(path)?;
        let raw = Self::read_and_follow_clusters(
            &self.partition,
            sector,
            self.fat_info.get_first_data_sector(),
            self.fat_info.first_fat_sector() as u64,
        )?;
        let content = String::from_utf8_lossy(&raw).to_string();
        Some(content)
    }
    pub fn read_dir(&self, path: &FilePath) -> Option<Vec<Fat32SoftEntry>> {
        let sector = self.get_sector(path)?;
        let sectors = Self::read_and_follow_clusters(
            &self.partition,
            sector,
            self.fat_info.get_first_data_sector(),
            self.fat_info.first_fat_sector() as u64,
        )?;
        let raw_entries_part = Self::get_raw_entries(&sectors);
        let entries_part = Self::parse_entries(
            &raw_entries_part,
            self.fat_info.get_first_data_sector(),
            path,
            &self.partition,
        );
        Some(entries_part)
    }
    /// Reads a fat32 entry and follow clusters from fat table
    pub fn read_and_follow_clusters(
        partition: &Partition,
        start_sector: u64,
        first_data_sector: u64,
        first_fat_sector: u64,
    ) -> Option<Vec<u8>> {
        let mut res = Vec::new();
        let mut reading = true;
        let mut next_sector = start_sector;
        while reading {
            let mut sector = read_from_partition(partition, next_sector, 1).unwrap();
            // if *sector.last().unwrap()==0 {
            if let Some(cluster) =
                Self::read_fat_cluster(partition, next_sector, first_fat_sector, first_data_sector)
            {
                match cluster {
                    ClusterEnum::EndOfChain => reading = false,
                    ClusterEnum::BadCluster => reading = false,
                    ClusterEnum::Cluster(cluster) => {
                        next_sector = cluster_to_sector(cluster as u64, first_data_sector);
                    }
                };
            } else {
                log::error!("Sector too small ?!");
                dbg!(Self::read_fat_cluster(
                    partition,
                    next_sector,
                    first_fat_sector,
                    first_data_sector
                ))
            }

            res.extend(sector);
            // next_sector+=1;
        }
        Some(res)
    }

    fn read_fat(
        partition: &Partition,
        fat_size: u32,
        first_fat_sector: u16,
        first_data_sector: u64,
    ) -> FatTable {
        let (mut last_sector, mut last_offset) = (0, 0);
        let mut last_used_sector = 0;
        for i in 0..fat_size {
            if last_sector != 0 {
                break;
            }
            let content =
                read_from_partition(partition, first_fat_sector as u64 + i as u64, 1).unwrap();
            for offset in 0..content.len() / 4 {
                let table_value = &content[offset * 4..offset * 4 + 4];
                let mut table_value = ((table_value[3] as u32) << 24)
                    | ((table_value[2] as u32) << 16)
                    | ((table_value[1] as u32) << 8)
                    | (table_value[0] as u32);
                table_value &= 0x0FFFFFFF;
                if table_value < 0x0FFFFFF8 && table_value != 0x0FFFFFF7 {
                    if table_value == 0 {
                        last_sector = first_fat_sector + i as u16;
                        last_offset = offset as u16;
                        // Useless to change table_value cuz never read after
                        // let table_value = &content[(offset - 1) * 4..(offset - 1) * 4 + 4];
                        // let mut table_value: u32 = ((table_value[3] as u32) << 24)
                        //     | ((table_value[2] as u32) << 16)
                        //     | ((table_value[1] as u32) << 8)
                        //     | (table_value[0] as u32);
                        // table_value &= 0x0FFFFFFF;
                        break;
                    }
                    last_used_sector =
                        cluster_to_sector(table_value as u64, first_data_sector) as u32;
                }
            }
        }
        FatTable {
            size: fat_size,
            first_sector: first_fat_sector,
            last_sector,
            last_offset,
            last_used_sector,
        }
    }
    // Current sector should be u32
    // Reads the fat table to know where is the next cluster to follow
    pub fn read_fat_cluster(
        partition: &Partition,
        current_sector: u64,
        first_fat_sector: u64,
        first_data_sector: u64,
    ) -> Option<ClusterEnum> {
        if current_sector < first_data_sector {
            // Should do current_sector-2 but we could have buffer underflows...
            log::error!(
                "Can't read cluster if sector is to small ! {}<{}",
                current_sector,
                first_data_sector
            );
            return None;
        }
        let fat_offset = ((current_sector - first_data_sector) + 2) * 4;
        let fat_sector = (fat_offset / 512) + first_fat_sector;
        let ent_offset = (fat_offset % 512) as usize;
        let content = read_from_partition(partition, fat_sector, 1).unwrap();
        let table_value = &content[ent_offset..ent_offset + 4];
        let mut table_value = ((table_value[3] as u32) << 24)
            | ((table_value[2] as u32) << 16)
            | ((table_value[1] as u32) << 8)
            | (table_value[0] as u32);
        table_value &= 0x0FFFFFFF;
        if table_value >= 0x0FFFFFF8 || table_value == 0 {
            Some(ClusterEnum::EndOfChain)
        } else if table_value == 0x0FFFFFF7 {
            Some(ClusterEnum::BadCluster)
        } else {
            Some(ClusterEnum::Cluster(table_value))
        }
    }
    fn get_fat_boot(partition: &Partition) -> Result<FatInfo, DiskError> {
        let raw_fat_boot = read_from_partition(partition, 0, 2)?;
        let fat_boot = unsafe { &*(raw_fat_boot.as_ptr() as *const BiosParameterBlock) };
        Ok(FatInfo(fat_boot.clone()))
    }
    //TODO Change prefix to String/&str ?
    fn walk_dir(
        prefix: FilePath,
        partition: &Partition,
        sector: u64,
        first_data_sector: u64,
        first_fat_sector: u64,
    ) -> HashMap<FilePath, Fat32SoftEntry> {
        let mut files = HashMap::new();
        let raw_sector =
            Self::read_and_follow_clusters(partition, sector, first_data_sector, first_fat_sector)
                .unwrap();
        let raw_entries = Self::get_raw_entries(&raw_sector);
        for entry in Self::parse_entries(&raw_entries, first_data_sector, &prefix, partition) {
            //TODO is_dir
            if !entry.is_file {
                let inner_entries = Self::walk_dir(
                    prefix.clone().join_str(entry.path.name().to_string()),
                    partition,
                    entry.sector,
                    first_data_sector,
                    first_fat_sector,
                );
                files.extend(inner_entries);
            }
            files.insert(entry.path.clone(), entry);
        }
        files
    }
    fn get_raw_entries(sector: &[u8]) -> Vec<RawFat32Entry> {
        let mut entries = Vec::new();
        for i in 0..sector.len() / 32 {
            let sector_section = &sector[(i * 32)..(i * 32) + 31];
            if sector_section[0] == 0 {
                break;
            } else if sector_section[0] != 0xE5 {
                let attribute = sector_section[11];
                let entry = if attribute == 0xF {
                    RawFat32Entry::LFN(
                        unsafe { &*(sector_section.as_ptr() as *const LFN32) }.clone(),
                    )
                } else {
                    RawFat32Entry::Standard(
                        unsafe { &*(sector_section.as_ptr() as *const Standard32) }.clone(),
                    )
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
    fn parse_entries(
        entries: &[RawFat32Entry],
        first_data_sector: u64,
        prefix: &FilePath,
        partition: &Partition,
    ) -> Vec<Fat32SoftEntry> {
        let mut files = Vec::new();
        let mut i = 0;
        while i < entries.len() {
            let entry = &entries[i];
            i += 1;
            match entry {
                RawFat32Entry::LFN(file) => {
                    let mut name = file.name();
                    let mut nentry = None;
                    loop {
                        if i >= entries.len() {
                            dbg!(file);
                            dbg!(entries[i - 1]);
                            break;
                        } else {
                            match &entries[i] {
                                RawFat32Entry::LFN(lfn) => {
                                    let mut new_name = lfn.name();
                                    new_name.push_str(&name);
                                    name = new_name;
                                    i += 1;
                                }
                                RawFat32Entry::Standard(file) => {
                                    nentry = Some(file);
                                    break;
                                }
                            };
                        }
                    }
                    if nentry.is_none() {
                        continue;
                    }
                    let nentry = nentry.unwrap();
                    let next_name = nentry.name();
                    if next_name.starts_with('.') || next_name.starts_with("..") {
                        continue;
                    } // Pass on "." and ".." folders
                    let is_file = nentry.attributes & 0x20 == 0x20;
                    let path = prefix.clone().join(FilePath::new(name, partition.clone()));

                    let fst_cluster = ((nentry.high_u16_1st_cluster as u32) << 16)
                        | (nentry.low_u16_1st_cluster as u32);
                    if fst_cluster > 1_000_000 {
                        serial_print!(
                            "Cluster to big ! {} {}",
                            fst_cluster,
                            nentry.printable(first_data_sector)
                        );
                    }
                    let sector = if fst_cluster > 2 {
                        cluster_to_sector(fst_cluster as u64, first_data_sector)
                    } else if fst_cluster == 0 {
                        // File is empty
                        0
                    } else {
                        log::error!("Cluster is too low ! ({})", fst_cluster);
                        dbg!(path);
                        i += 1;
                        continue;
                    } as u32;

                    let parsed_entry = Fat32SoftEntry {
                        path,
                        sector: sector as u64,
                        is_file,
                    };
                    files.push(parsed_entry);

                    i += 1; // Skip next entry cuz it's related to the current LFN
                }
                RawFat32Entry::Standard(file) => {
                    // From my tests only "." and ".." folders
                    //EDIT Also CACHEDIRTAG file
                    if !file.name().starts_with('.') && !file.name().contains("CACHEDIRTAG") {
                        log::debug!("What is this file ?"); // If this prints one day we need to do investigations
                        dbg!(file)
                    }
                }
            }
        }
        files
    }
}
impl FsDriver for Fat32Driver {
    fn as_enum(&self) -> FsDriverEnum {
        FsDriverEnum::Fat32
    }
    fn read(&self, path: &FilePath) -> Result<Entry, FsReadError> {
        let soft_entry = self.files.get(path).ok_or(FsReadError::EntryNotFound)?;
        let entry = match soft_entry.is_file {
            true => {
                let content = self.read_file(path).unwrap(); // Safe unwrap cuz we know file exists from above
                let size = content.len();
                Entry::File(File {
                    path: soft_entry.path.clone(),
                    content,
                    size,
                })
            }
            false => {
                let entries: Vec<SoftEntry> = self
                    .read_dir(path)
                    .unwrap()
                    .into_iter()
                    .map(|entry| SoftEntry {
                        path: entry.path,
                        size: 0,
                    })
                    .collect();
                Entry::Dir(Dir {
                    path: soft_entry.path.clone(),
                    size: entries.len(),
                    entries,
                })
            }
        };
        Ok(entry)
    }
    fn partition(&self) -> &Partition {
        &self.partition
    }
}
impl FsDriverInitialiser for Fat32Driver {
    fn try_init(partition: &Partition) -> Option<Box<Self>>
    where
        Self: Sized,
    {
        Some(Box::new(Self::new(partition)?))
    }
}

#[derive(Debug)]
pub struct Fat32SoftEntry {
    pub path: FilePath,
    pub sector: u64,
    pub is_file: bool,
}

#[derive(Debug, Clone)]
pub struct Fat32File {
    pub path: FilePath,
    pub size: u64,
    pub attributes: FatAttributes,
    pub sector: u32,
}
impl Fat32File {
    pub fn path(&self) -> &FilePath {
        &self.path
    }
    pub fn name(&self) -> &str {
        self.path.name()
    }
    pub fn sector(&self) -> u32 {
        self.sector
    }
    pub fn attributes(&self) -> &FatAttributes {
        &self.attributes
    }
}
#[derive(Debug, Clone)]
pub struct Fat32Dir {
    pub path: FilePath,
    pub attributes: FatAttributes,
    pub sector: u32,
    // pub dirs: HashMap<FilePath, Fat32Dir>,
}
impl Fat32Dir {
    pub fn path(&self) -> &FilePath {
        &self.path
    }
    pub fn name(&self) -> &str {
        self.path.name()
    }
    pub fn sector(&self) -> u32 {
        self.sector
    }
    // pub fn attributes(&self) -> &FatAttributes {
    //     &self.attributes
    // }
}

//TODO Mult by sectors_per_cluster
// All safely to u32
pub fn cluster_to_sector(cluster_number: u64, first_data_sector: u64) -> u64 {
    (cluster_number - 2) + first_data_sector
}
pub fn sector_to_cluster(sector_number: u64, first_data_sector: u64) -> u64 {
    (sector_number - first_data_sector) + 2
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(packed)]
pub struct BiosParameterBlock {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
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

pub enum FatType {
    ExFat,
    Fat12,
    Fat16,
    Fat32,
}
#[derive(Default, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FatInfo(pub BiosParameterBlock);
impl FatInfo {
    pub fn first_sector_of_cluster(&self) -> u64 {
        let first_data_sector = self.get_first_data_sector();
        cluster_to_sector(self.0.root_dir_first_cluster as u64, first_data_sector)
        // , self.0.sectors_per_cluster as u64
    }
    pub fn get_first_data_sector(&self) -> u64 {
        let fat_size = self.get_fat_size();
        let root_dir_sectors = self.get_root_dir_sectors();
        let reserved_sector_count = self.0.reserved_sectors;
        reserved_sector_count as u64 + (self.0.fats as u64 * fat_size as u64) + root_dir_sectors
    }
    pub fn fat_type(&self) -> FatType {
        let total_clusters = self.get_total_clusters();
        if total_clusters < 4085 {
            FatType::Fat12
        } else if total_clusters < 65525 {
            FatType::Fat16
        } else {
            FatType::Fat32
        }
    }
    pub fn get_total_clusters(&self) -> u64 {
        self.get_data_sectors() / self.0.sectors_per_cluster as u64
    }
    pub fn get_data_sectors(&self) -> u64 {
        self.get_total_sectors() as u64
            - (self.0.reserved_sectors as u64
                + (self.0.fats as u64 * self.get_fat_size() as u64)
                + self.get_root_dir_sectors())
    }
    pub fn get_total_sectors(&self) -> u32 {
        if self.0.total_sectors_16 == 0 {
            self.0.total_sectors_32
        } else {
            self.0.total_sectors_16.into()
        }
    }
    // Gets fat size in sectors
    pub fn get_fat_size(&self) -> u32 {
        if self.0.sectors_per_fat_16 == 0 {
            self.0.sectors_per_fat_32
        } else {
            self.0.sectors_per_fat_16 as u32
        }
    }
    pub fn get_root_dir_sectors(&self) -> u64 {
        ((self.0.root_entries as u64 * 32_u64) + (self.0.bytes_per_sector as u64 - 1))
            / self.0.bytes_per_sector as u64
    }
    pub fn first_fat_sector(&self) -> u16 {
        self.0.reserved_sectors
    }
}

#[derive(Debug, Default)]
pub struct FatTable {
    pub size: u32,
    pub first_sector: u16,
    pub last_sector: u16,
    pub last_offset: u16, // u16 even though in range 0..512
    pub last_used_sector: u32,
}
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Clone, Copy)]
pub enum ClusterEnum {
    EndOfChain,
    BadCluster,
    Cluster(u32),
}

#[derive(Default, Clone)]
#[repr(C, packed)]
pub struct Standard32 {
    // r:u8,
    pub name: [u8; 8],
    pub extension: [u8; 3],
    pub attributes: u8,
    pub reserved: u8,
    pub duration_creation_time: u8,
    pub creation_time: u16,
    pub creation_date: u16,
    pub last_accessed_date: u16,
    pub high_u16_1st_cluster: u16,
    pub last_modif_time: u16,
    pub last_modif_date: u16,
    pub low_u16_1st_cluster: u16,
    pub size: u32,
}
impl Standard32 {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(
            [self.name.to_vec(), self.extension.to_vec()]
                .concat()
                .as_slice(),
        )
        .to_string()
    }
    pub fn printable(&self, first_data_sector: u64) -> String {
        let creation_date = self.creation_date;
        let fst_cluster =
            ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let sector = if fst_cluster > 2 {
            (fst_cluster as u64 - 2) + first_data_sector
        } else {
            0
        };
        let size = self.size;
        let _entry_type = self.name[0];
        let name = self.name();
        format!(
            "File8.3: {}\t | creation_date: {} | 1st cluster: {}({}) \t| size: {}\t| attrs: {:#b}",
            name, creation_date, fst_cluster, sector, size, self.attributes
        )
    }
}
impl core::fmt::Debug for Standard32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let creation_date = self.creation_date;
        let fst_cluster =
            ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let size = self.size;
        let _entry_type = self.name[0];
        let _name = self.name();
        f.write_str(format!("Standard8.3Entry({} creation_date: {} | 1st cluster: {} \t| size: {}\t| attrs: {:#b})", self.name(), creation_date, fst_cluster, size, self.attributes).as_str())
    }
}

#[derive(Clone)]
#[repr(C, packed)]
pub struct LFN32 {
    order: u8, // The order of this entry in the sequence of long file name entries. This value helps you to know where in the file's name the characters from this entry should be placed.
    fst_chars: [u16; 5],
    attribute: u8,       // Should ALWAYS be 0xF
    long_entry_type: u8, // Zero for name entries
    chksum: u8, // Checksum generated of the short file name when the file was created. The short filename can change without changing the long filename in cases where the partition is mounted on a system which does not support long filenames
    scd_chars: [u16; 6],
    zeroes: u16,
    fin_chars: [u16; 2],
}

impl LFN32 {
    pub fn name(&self) -> String {
        let fst_chars = self.fst_chars; // To get packed fields
        let scd_chars = self.scd_chars;
        let fin_chars = self.fin_chars;
        let raw_name = [fst_chars.to_vec(), scd_chars.to_vec(), fin_chars.to_vec()].concat();

        let mut name = String::new();
        for chr in raw_name {
            if chr == 0 || chr == 255 {
                continue;
            }
            name.push_str(String::from_utf16_lossy(&[chr]).to_string().as_str());
        }
        name
        // String::from_utf16_lossy(&raw_name).to_string()
    }
}
impl core::fmt::Debug for LFN32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("LFN32({})", self.name()).as_str())
    }
}
#[derive(Debug, Clone)]
pub enum RawFat32Entry {
    LFN(LFN32),
    Standard(Standard32),
}

// pub fn write_file(
//     &mut self,
//     _path: impl Into<FilePath>,
//     _content: String,
// ) -> Result<(), FileSystemError> {
//     todo!()
// }
// pub fn write_dir(&mut self, path: impl Into<FilePath>) -> Result<(), FileSystemError> {
//     let path = path.into();
//     if let Some(parent_entry) = self.files.get(&Into::<FilePath>::into(path.parent())) {
//         let start_sector = match parent_entry {
//             Entry::File(_) => todo!(),
//             Entry::Dir(dir) => dir.sector,
//         };
//         dbg!(start_sector);
//         let path = Into::<FilePath>::into(path);
//         let mut path_name: Vec<u8> = path.name().bytes().collect();
//         if path_name.len() < 11 {
//             // Add some 0 to convert to [u8; 11]
//             for _i in 0..11 - path_name.len() {
//                 path_name.push(0);
//             }
//         }
//         let name = <[u8; 8]>::try_from(path_name.clone()).unwrap();
//         let extension = <[u8; 3]>::try_from(path_name[8..].to_vec()).unwrap();
//         let size = 1;
//         let to_write_sector = self.fat_table.last_sector + 1;
//         dbg!(to_write_sector);
//         dbg!(self.fat_info.get_first_data_sector());
//         let cluster = sector_to_cluster(
//             to_write_sector as u64,
//             self.fat_info.get_first_data_sector(),
//         );
//         let entry = Standard32 {
//             name,
//             extension,
//             high_u16_1st_cluster: TryInto::<u16>::try_into(cluster & 0xFF00).unwrap(),
//             low_u16_1st_cluster: TryInto::<u16>::try_into(cluster & 0x00FF).unwrap(),
//             size,
//             ..core::default::Default::default()
//         };
//         let content = unsafe { any_as_u8_slice(&entry) };
//         serial_println!(
//             "Writing {:?} - {:?}",
//             content,
//             String::from_utf8_lossy(content)
//         );
//         let mut bytes = Vec::new();
//         for c in content {
//             bytes.push(*c);
//         }
//         if let Err(_) = write_to_partition(&self.partition, start_sector as u64, bytes) {
//             return Err(FileSystemError::CantWrite)
//         }
//         let sector = 0;
//         self.files.insert(
//             path.clone(),
//             Entry::Dir(Fat32Dir {
//                 path,
//                 attributes: FatAttributes::default(),
//                 sector,
//             }),
//         );
//     } else {
//         log::error!(
//             "whilst trying to write dir in {} (parent: {})",
//             path,
//             path.parent()
//         );
//     }
//     Ok(())
// }
