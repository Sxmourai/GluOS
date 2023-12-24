use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{cell::Cell, convert::Infallible, num::TryFromIntError};
use log::debug;
use spin::Mutex;

pub mod ata;

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
