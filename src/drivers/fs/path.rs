use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::{disk::DiskLoc, fs_driver};

use super::{partition::Partition, userland::FatAttributes};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
pub enum FileSystemError {
    FileNotFound,
    CantWrite,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Hash, Debug)]
pub struct FilePath {
    raw_path: String,
    pub partition: Partition,
}
impl FilePath {
    pub fn new(mut full_path: String, partition: Partition) -> Self {
        if !full_path.starts_with('/') {
            full_path.insert(0, '/');
        }
        Self {
            raw_path: full_path.replace('\u{ffff}', ""),
            partition,
        }
    }
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u64 {
        let mut len = 0;
        for _word in self.raw_path.split('/') {
            len += 1;
        }
        len
    }
    pub fn root(&self) -> &str {
        let mut splitted = self.raw_path.split('/');
        splitted.next().unwrap()
    }
    /// Creates a new filepath poiting to parent
    pub fn parent(&self) -> FilePath {
        let splitted: Vec<&str> = self.raw_path.split('/').collect();
        FilePath::new(
            splitted[0..splitted.len() - 2].join("/"),
            self.partition.clone(),
        )
    }
    pub fn path(&self) -> &String {
        &self.raw_path
    }
    // Both paths must be on same partition !
    pub fn join(&self, other_path: FilePath) -> FilePath {
        let mut path = self.raw_path.clone();
        assert_eq!(self.partition, other_path.partition);
        path.push_str(other_path.path());
        Self::new(
            path.replace("//", "/").replace('\\', "/"),
            self.partition.clone(),
        )
    }
    //TODO Return new or mutate self ?
    pub fn join_str(&self, other_path: String) -> FilePath {
        let path = &format!("{}/{}", self.path(), other_path);
        Self::new(
            path.replace("//", "/").replace('\\', "/"),
            self.partition.clone(),
        )
    }
    pub fn disk_loc(&self) -> DiskLoc {
        self.partition.0
    }
    pub fn name(&self) -> &str {
        self.raw_path.split('/').last().unwrap()
    }
}
impl core::fmt::Display for FilePath {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.path().to_string().as_str())
    }
}
