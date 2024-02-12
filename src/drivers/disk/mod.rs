pub mod ata;
pub mod driver;
pub mod nvme;

pub fn init() {
    let mut disks = hashbrown::HashMap::new();
    for (loc, device) in crate::pci_manager!().iter() {
        if device.class.id() != 0x1 {continue}
        if device.subclass() == 0x1 {
            crate::trace!("Found IDE controller on bus {loc}");
            for (i, disk) in ata::init().into_iter().enumerate() {
                disks.insert(DiskLoc::from_idx(i.try_into().unwrap()).unwrap(), disk);
            }
        } else if device.subclass() == 0x8 {
            crate::trace!("Found NVMe controller on bus {loc}");
            if let Some(nvme_disks) = nvme::init(device) {
                for (i, disk) in nvme_disks.into_iter().enumerate() {
                    disks.insert(DiskLoc::from_idx(i.try_into().unwrap()).unwrap(), driver::Disk {
                        loc: DiskLoc(Channel::Secondary, Drive::Slave),
                        drv: driver::DiskDriverEnum::NVMe,
                    });
                }
            } else {
                log::error!("Failed initialising NVMe driver")
            }
        }
    }
    unsafe{DISK_MANAGER.lock().replace(DiskManager {
        disks,
    })};
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum DiskError {
    NotFound,
    ReadDataNotAvailable,
    PermissionDenied, // Shouldn't happen... But keep this for rlib ?
    SectorTooBig,
    NoReadModeAvailable,
    DiskNotFound,
    TimeOut,
    DRQRead, //TODO Handle all errors from the register
}
use crate::disk::{ata::{Channel, Drive}, driver::GenericDisk};

use self::driver::{DiskManager, DISK_MANAGER};
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct DiskLoc(pub Channel, pub Drive);
impl DiskLoc {
    fn as_index(&self) -> usize {
        let mut i = 0;
        if self.0 == Channel::Secondary {
            i += 2
        }
        if self.1 == Drive::Slave {
            i += 1
        }
        i
    }
    #[cfg(feature = "fs")]
    fn as_path(
        &self,
        partition: crate::fs::partition::Partition,
    ) -> Option<crate::fs::path::FilePath> {
        use alloc::string::ToString;

        Some(crate::fs::path::FilePath::new("/".to_string(), partition))
    }
    fn as_diskloc(&self) -> DiskLoc {
        DiskLoc(self.channel(), self.drive())
    }
    fn channel(&self) -> Channel {
        match self.as_index() {
            0 => Channel::Primary,
            1 => Channel::Primary,
            2 => Channel::Secondary,
            3 => Channel::Secondary,
            _ => panic!("Invalid channel address"),
        }
    }
    fn drive(&self) -> Drive {
        match self.as_index() {
            0 => Drive::Master,
            1 => Drive::Slave,
            2 => Drive::Master,
            3 => Drive::Slave,
            _ => panic!("Invalid drive address"),
        }
    }
    fn channel_addr(&self) -> u16 {
        self.channel() as u16
    }
    fn drive_select_addr(&self) -> u8 {
        match self.drive() {
            Drive::Master => 0xA0,
            Drive::Slave => 0xB0,
        }
    }
    fn drive_lba28_addr(&self) -> u8 {
        match self.drive() {
            Drive::Master => 0xE0,
            Drive::Slave => 0xF0,
        }
    }
    fn drive_lba48_addr(&self) -> u8 {
        match self.drive() {
            Drive::Master => 0x40,
            Drive::Slave => 0x50,
        }
    }
    fn base(&self) -> u16 {
        self.channel_addr()
    }

    pub fn from_idx(idx: u8) -> Option<Self> {
        Some(match idx {
            0 => Self(Channel::Primary, Drive::Master),
            1 => Self(Channel::Primary, Drive::Slave),
            2 => Self(Channel::Secondary, Drive::Master),
            3 => Self(Channel::Secondary, Drive::Slave),
            _ => return None,
        })
    }
}
