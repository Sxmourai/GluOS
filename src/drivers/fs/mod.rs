pub mod entry;
pub mod fs;
pub mod fs_driver;
pub mod userland;
pub mod partition;
// Specific fs's
pub mod ext;
pub mod fat;
pub mod ntfs;


use alloc::{boxed::Box, vec::Vec};
use hashbrown::HashMap;

use crate::{dbg, disk::ata::{DiskLoc, DISK_MANAGER}, fs_driver, state::FS_DRIVER};

use self::{fs::FilePath, fs_driver::{Entry, FsDriver, FsDriverInitialiser, FsReadError}, partition::{HeaderType, Partition}};

/// Holds drivers for all of the partitions of all the disks
pub struct FsDriverManager {
    pub drivers: HashMap<Partition, Box<dyn FsDriver>>,
    pub partitions: HashMap<DiskLoc, Vec<Partition>>
}
impl FsDriverManager {
    pub fn read(&self, path: &FilePath) -> Result<Entry, FsReadError> {
        if let Some(driver) = self.drivers.get(&path.partition) {
            driver.read(path)
        } else {
            dbg!(self.drivers);
            Err(FsReadError::EntryNotFound)
        }
    }
    pub fn get_partition_from_id(&self, loc: &DiskLoc, part_id: u8) -> Option<&Partition> {
        self.partitions.get(loc)?.get(part_id as usize)
    }
    pub fn new() -> Self {
        let mut self_drivers = HashMap::new();
        let mut self_partitions = HashMap::new();
        for (i,disk) in unsafe{&DISK_MANAGER.lock().as_mut().unwrap().disks}.iter().enumerate() {
            if let Some(disk) = disk {
                let header_type = partition::read_header_type(disk);
                if header_type.is_none() {continue}
                let header_type = header_type.unwrap();
                let partitions = match header_type {
                    HeaderType::GPT(gpt) => gpt,
                    HeaderType::MBR(mbr) => mbr,
                };
                self_partitions.insert(disk.loc, partitions);
            }
        }
        for (disk, parts) in &self_partitions {
            for part in parts {
                let drv = partition::find_and_init_fs_driver_for_part(part);
                if drv.is_none() {
                    log::error!("Couldn't init a fs driver on partition {:?}", part);
                    continue
                }
                let drv = drv.unwrap();
                self_drivers.insert(part.clone(), drv);
            }
        }

        Self {
            drivers: self_drivers,
            partitions: self_partitions,
        }
    }
}

pub fn get_fs_driver(loc: &Partition) -> Option<&Box<dyn FsDriver>> {
    unsafe{fs_driver!().drivers.get(loc)}
}

pub fn init() {
    unsafe { FS_DRIVER.replace(FsDriverManager::new()) };
}