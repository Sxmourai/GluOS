use core::cell::Cell;

use self::fs::{Fat32File, FilePath, GenericFile, FileSystemError};

use super::{Driver, DriverError};

pub mod fat_driver;
pub mod fs;

use alloc::{boxed::Box, vec::Vec};
use hashbrown::HashMap;
use spin::{Mutex, MutexGuard};


pub fn fat_driver() -> Mutex<Box<FsDriver>> {
    *get_driver("FAT").unwrap()
}
pub type Files = HashMap<FilePath, Box<Fat32File>>;
#[derive(Default)]
pub struct FsDriver {
    opened_files: Files,
    initialised: bool,
}
impl FsDriver {
    pub fn open_file(&mut self, file_path: &FilePath) -> Result<GenericFile, FileSystemError> {
        todo!()
    }
    pub fn close_file(&mut self, file: &mut Fat32File) -> Result<(), FileSystemError> {
        if (self.opened_files.get(file.path()).is_some()) {
            self.opened_files.remove(file.path());
            Ok(())
        } else {
            Err(FileSystemError::FileNotFound)
        }
    }
    pub fn ls(&self, path: &FilePath) -> &Files {
        &self.opened_files
    }
}
impl Driver for FsDriver {
    fn init(&mut self) -> Result<(), super::DriverError> {
        if self.initialised {
            Err(DriverError::AlreadyExists)
        } else {
            Ok(())
        }
    }
}
