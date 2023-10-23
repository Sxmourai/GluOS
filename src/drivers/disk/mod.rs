use core::{cell::Cell, num::TryFromIntError, convert::Infallible};
use alloc::{string::{String, ToString}, vec::Vec};
use log::debug;
use spin::Mutex;


use super::{Driver, DriverError, get_driver};

pub mod fs;
pub mod ata;

pub struct DiskDriver {}
impl Driver for DiskDriver {
    fn name(&self) -> &str {"Disk"}

    fn init(&mut self) -> Result<(), DriverError> {
        debug!("Init ATA driver");
        ata::init();
        Ok(())
    }

    fn required(&self) -> &str {
        "Memory && Time"
    }

    fn new() -> Self where Self: Sized {
        Self {
            
        }
    }
}


#[derive(Debug)]
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

impl From<TryFromIntError> for DiskError {
    fn from(original: TryFromIntError) -> Self {
        DiskError::SectorTooBig
    }
}
// impl Into<String> for DiskError {
//     fn into(self) -> String {
//         match self {
//             DiskError::NotFound => "Not found",
//             DiskError::PermissionDenied => "Permission denied",
//             DiskError::SectorTooBig => "Sector too big",
//             DiskError::NoReadModeAvailable => "No read mode available on disk",
//             DiskError::DiskNotFound => "Disk not found",
//             DiskError::TimeOut => "Time out",
//             DiskError::DRQRead => "DRQ read error",
//         }.to_string()
//     }
// }
impl From<DiskError> for String {
    fn from(original: DiskError) -> Self {
        Self::new()
    }
}