use alloc::{boxed::Box, vec::Vec};

use crate::{dbg, disk::ata::{Disk, DiskLoc}, fs_driver};

use super::{fat::Fat32Driver, fs_driver::FsDriver, get_fs_driver};

#[repr(C, packed)]
#[derive(Debug)]
pub struct MBRPartition {
    pub drive_attribute: u8, // Drive attributes (bit 7 set = active or bootable)
    pub chs_addr: [u8; 3], // CHS Address of partition start
    pub partition_type: u8,
    pub chs_end_addr: [u8; 3], // CHS address of last partition sector
    pub lba_start: u32, // LBA of partition start
    pub sector_count: u32, // Number of sectors in partition
}
#[repr(C, packed)]
#[derive(Debug)]
pub struct GPTPartition {
    pub part_type_guid: [u8; 16], // zero means unused entry
    pub unique_guid: [u8; 16],
    pub start_lba: u64,
    pub end_lba: u64,
    pub attributes: u64,
    pub name: [u8; 72],
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// Holds a disk loc, the start offset of the partition & size
pub struct Partition(pub DiskLoc, pub u64, pub u64);
impl Partition {
    pub fn from_idx(loc: &DiskLoc, part_id: u8) -> Option<&Self> {
        unsafe{fs_driver!().get_partition_from_id(loc, part_id)}
    }
}

pub type GPTHeader = ();
//TODO Implement proper GPT/MBR parsing and checksuming & all
#[derive(Debug)]
pub enum HeaderType {
    GPT(Vec<Partition>),
    MBR(Vec<Partition>),
}


pub const MBR_SIGNATURE: [u8; 2] = [0x55, 0xAA];
pub const GPT_SIGNATURE: [u8; 8] = [69, 70, 73, 32, 80, 65, 82, 84];

pub fn read_header_type(disk: &Disk) -> Option<HeaderType> {
    // Check GPT
    //TODO Remove all of this nesting
    if let Ok(sec_sector) = disk.read_sectors(1, 1) {
        if sec_sector[0..GPT_SIGNATURE.len()] == GPT_SIGNATURE {
            let mut partitions = Vec::new();
            for sector in 2..33 {
                let raw_partitions = disk.read_sectors(sector, 1);
                if raw_partitions.is_err() {break}
                let raw_partitions = raw_partitions.unwrap();
                for part_num in 0..4 { // 128 bytes per partition, and 1 sector = 512 bytes
                    let partition = unsafe{ &*(raw_partitions[128*part_num..].as_ptr() as *const GPTPartition) };
                    // Check if only zeroes, if so done reading ?
                    if partition.part_type_guid.into_iter().all(|x| x==0) {break}
                    let start_lba = partition.start_lba;
                    let end_lba = partition.end_lba;
                    partitions.push(Partition(disk.loc.clone(), partition.start_lba, partition.end_lba));
                }
            }
            return Some(HeaderType::GPT(partitions))
        }
    }
    // Check MBR
    if let Ok(first_sector) = disk.read_sectors(0, 1) {
        if first_sector[first_sector.len()-MBR_SIGNATURE.len()..] == MBR_SIGNATURE {// https://wiki.osdev.org/MBR_(x86)#MBR_Format
            let mut partitions = Vec::new();
            for part_num in 0..4 {
                let mbr_part = unsafe{ &*(first_sector[446+(16*part_num)..].as_ptr() as *const MBRPartition) };
                if first_sector[446+(16*part_num)..446+(16*part_num)+16].iter().all(|x| *x==0) {
                    continue
                }
                let lba_start = mbr_part.lba_start;
                let sector_count = mbr_part.sector_count;
                partitions.push(Partition(disk.loc.clone(), mbr_part.lba_start as u64, mbr_part.sector_count as u64));
            }
            return Some(HeaderType::MBR(partitions))
        }
    }
    // No MBR / GPT On disk (raw contents then or maybe not even a disk ?!)
    crate::dbg!("No MBR/GPT on partition ?", disk);
    None
}
pub struct _FsDriverWrapper<'a>(pub &'a Partition);
impl _FsDriverWrapper<'_> {
    pub fn try_init_drv<T: FsDriver>(&self) -> Option<Box<T>> {
        T::try_init(self.0)
    }
}

macro_rules! fs_driver_init {
    ($drv: ty, $part: ident) => {
        $drv::try_init($part)
    };
}

pub fn find_and_init_fs_driver_for_part(part: &Partition) -> Option<Box<dyn FsDriver>> {
    if let Some(drv) = _FsDriverWrapper(part).try_init_drv::<Fat32Driver>() {
        return Some(drv);
    }
    if let Some(drv) = _FsDriverWrapper(part).try_init_drv::<super::ext::ExtDriver>() {
        return Some(drv);
    }
    if let Some(drv) = _FsDriverWrapper(part).try_init_drv::<super::ntfs::NTFSDriver>() {
        return Some(drv);
    }
    None
}
//let fat_info = Fat32Driver::get_fat_boot(&partition).unwrap();
// if fat_info.0.fs_type_label[0..5] == [70, 65, 84, 51, 50] {
//     // Fat32
// } else if fat_info.0.fs_type_label.iter().all(|x|*x==0) {
//     let driver = ExtDriver::new(&partition).unwrap();
// } else {
//     let name = String::from_utf8_lossy(&fat_info.0.fs_type_label.to_vec()).to_string();
//     log::error!("Unknown fs: {name}");
// }
// let first_fat_sector = fat_info.first_fat_sector();
// let first_data_sector = fat_info.get_first_data_sector();
// let fat_table = FsDriver::read_fat(
//     &partition,
//     fat_info.get_fat_size(),
//     first_fat_sector,
//     first_data_sector,
// );
// crate::dbg!(part_id);
// let files = Self::read_dirs_structure(&fat_info, &partition, &part_id.as_path().unwrap()).unwrap();