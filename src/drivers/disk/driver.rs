use core::fmt::{Debug, Display};

use alloc::{boxed::Box, vec::Vec};
use hashbrown::HashMap;
use spin::Mutex;

use super::{
    ata::{AtaDisk, ATA_DRIVER},
    DiskError, DiskLoc,
};

pub static mut DISK_MANAGER: Mutex<Option<DiskManager>> = Mutex::new(None); // Uninitialised
pub const SECTOR_SIZE: u16 = 512;

#[macro_export]
macro_rules! disk_manager {
    () => {
        unsafe {
            $crate::drivers::disk::driver::DISK_MANAGER
                .lock()
                .as_mut()
                .unwrap()
        }
    };
}
#[derive(Debug)]
pub enum DiskDriverType {
    IDE,
    NVMe,
}

#[derive(Debug)]
pub struct DiskManager {
    /// Values are the GenericDisk and the index of the driver to use to read the disk
    pub disks: HashMap<DiskLoc, Disk>,
}
#[derive(Debug)]
pub struct Disk {
    pub loc: DiskLoc,
    pub drv: DiskDriverEnum,
}
#[derive(Debug)]
pub enum DiskDriverEnum {
    Ata,
    NVMe, // Not supported for now
}

impl DiskManager {
    pub fn read_disk(
        &mut self,
        loc: &DiskLoc,
        start_sector: u64,
        sector_count: u64,
    ) -> Result<Vec<u8>, DiskError> {
        match self.disks.get(loc).ok_or(DiskError::NotFound)?.drv {
            DiskDriverEnum::Ata => {
                let mut ata_drv = unsafe { ATA_DRIVER.as_mut().unwrap().write() };
                ata_drv.read(loc, start_sector, sector_count)
            }
            DiskDriverEnum::NVMe => {
                todo!()
            }
        }
    }
    pub fn write_disk(
        &mut self,
        loc: &DiskLoc,
        start_sector: u64,
        content: &[u8],
    ) -> Result<(), DiskError> {
        match self.disks.get(loc).ok_or(DiskError::NotFound)?.drv {
            DiskDriverEnum::Ata => {
                let mut ata_drv = unsafe { ATA_DRIVER.as_mut().unwrap().write() };
                ata_drv.write(loc, start_sector, content)
            }
            DiskDriverEnum::NVMe => {
                todo!()
            }
        }
    }
}

pub fn read_from_disk(
    addr: &DiskLoc,
    start_sector: u64,
    sector_count: u64,
) -> Result<Vec<u8>, DiskError> {
    disk_manager!().read_disk(addr, start_sector, sector_count)
}
#[cfg(feature = "fs")]
use crate::fs::partition::Partition;
#[cfg(feature = "fs")]
pub fn read_from_partition(
    partition: &Partition,
    start_sector: u64,
    sector_count: u64,
) -> Result<Vec<u8>, DiskError> {
    let start_sector = start_sector + partition.1;
    assert!(
        (start_sector + sector_count as u64) < partition.1 + partition.2,
        "Trying to read outside of partition"
    );
    read_from_disk(&partition.0, start_sector, sector_count)
}
pub fn write_to_disk(addr: &DiskLoc, start_sector: u64, content: &[u8]) -> Result<(), DiskError> {
    disk_manager!().write_disk(addr, start_sector, content)
}
#[cfg(feature = "fs")]
pub fn write_to_partition(
    partition: &Partition,
    start_sector: u64,
    content: &[u8],
) -> Result<(), DiskError> {
    let start_sector = start_sector + partition.1;
    assert!(
        (start_sector + content.len() as u64) < partition.1 + partition.2,
        "Trying to write outside of partition"
    );
    disk_manager!().write_disk(&partition.0, start_sector, content)
}

pub trait DiskDriver: Debug {
    fn read(
        &mut self,
        loc: &DiskLoc,
        start_sector: u64,
        sector_count: u64,
    ) -> Result<Vec<u8>, DiskError>;
    fn write(&mut self, loc: &DiskLoc, start_sector: u64, content: &[u8]) -> Result<(), DiskError>;
    fn select_disk(&mut self, disk: &DiskLoc);
}

pub trait GenericDisk: core::fmt::Debug + Display {
    fn loc(&self) -> &DiskLoc;
}
