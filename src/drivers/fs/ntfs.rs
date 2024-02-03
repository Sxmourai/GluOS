
use alloc::{format, string::{String, ToString}, vec::Vec};
use hashbrown::HashMap;
use ntfs::{Ntfs, NtfsFile, NtfsReadSeek};

use crate::{bit_manipulation::all_zeroes, dbg, disk::driver::read_from_partition, fs::fs_driver::{Dir, SoftEntry}, println, serial_println};

use super::{path::FilePath, fs_driver::{Entry, File, FsDriver, FsDriverInitialiser, FsReadError}, partition::Partition};

#[derive(Debug)]
pub struct NTFSDriver {
    partition: Partition,
    ntfs: ntfs::Ntfs,
    io: DiskReader,
    files: HashMap<FilePath, Entry>
}

impl FsDriverInitialiser for NTFSDriver {
    fn try_init(partition: &Partition) -> Option<alloc::boxed::Box<Self>> where Self: Sized {
        let mut reader = DiskReader {
            partition: partition.clone(),
            pos: 0,
        };
        let mut drv = ntfs::Ntfs::new(&mut reader).ok()?;
        drv.read_upcase_table(&mut reader).ok()?;
        let root = drv.root_directory(&mut reader).ok()?;
        let files = Self::walk_dir(partition, "", &mut reader, &drv, root)?;
        Some(alloc::boxed::Box::new(Self {
            partition: partition.clone(),
            ntfs: drv,
            io: reader,
            files,
        }))
    }
}

impl NTFSDriver {
    fn walk_dir<'a>(partition: &Partition, prefix: &str, reader: &'a mut DiskReader,ntfs: &Ntfs, dir: NtfsFile<'a>) -> Option<HashMap<FilePath, Entry>> {
        let mut parsed_entries = HashMap::new();
        let mut entries_idx = dir.directory_index(reader).ok()?;
        let mut entries = entries_idx.entries();
        while let Some(Ok(mut entry)) = entries.next(reader) {
            let file_name = entry.key().unwrap().unwrap();
            // println!("{}", file_name.name());
            if let Ok(name) = file_name.name().to_string() {
                if name.starts_with('$') { // Skip if starts with $
                    if let Some(Ok(_entry)) = entries.next(reader) {
                        entry = _entry;
                        continue
                    } else {
                        break
                    }
                }
                if let Ok(file) = entry.to_file(ntfs, reader) {
                    let parsed_entry = if file.is_directory() {
                        let a = Self::walk_dir(partition, file_name.name().to_string_lossy().as_str(), reader, ntfs, file.clone())?;
                        let mut soft_entries = Vec::new();
                        for (path, soft_entry) in a.iter() {
                            soft_entries.push(SoftEntry {
                                path: path.clone(),
                                size: 0,
                            })
                        }
                        parsed_entries.extend(a);
                        Entry::Dir(Dir {
                            path: FilePath::new(format!("{}/{}",prefix, file_name.name()), partition.clone()),
                            entries: soft_entries,
                            size: file.data_size() as usize,
                        })
                    } else if let Some(Ok(b)) = file.data(reader, file_name.name().to_string_lossy().as_str()) {
                        let v = b.to_attribute().unwrap();
                        let mut buf = Vec::new();
                        v.value(reader).unwrap().read(reader, &mut buf).unwrap();
                        let content = String::from_utf8_lossy(&buf).to_string();
                        Entry::File(File {
                            path: FilePath::new(format!("{}/{}",prefix, file_name.name()), partition.clone()),
                            content,
                            size: file.data_size() as usize,
                        })
                    } else {continue};
                    parsed_entries.insert(FilePath::new(name, partition.clone()), parsed_entry);
                }
            }
        }
        Some(parsed_entries)
    }
}

impl FsDriver for NTFSDriver {
    fn read(&self, path: &FilePath) -> Result<super::fs_driver::Entry, super::fs_driver::FsReadError> {
        dbg!(self.files);
        Ok(self.files.get(path).ok_or(FsReadError::EntryNotFound)?.clone())
    }

    fn as_enum(&self) -> super::fs_driver::FsDriverEnum {
        super::fs_driver::FsDriverEnum::NTFS
    }

    fn partition(&self) -> &super::partition::Partition {
        &self.partition
    }
}

#[derive(Debug)]
pub struct DiskReader {
    partition: Partition,
    pos: u64, // In bytes
}
impl binrw::io::Read for DiskReader {
    fn read(&mut self, buf: &mut [u8]) -> binrw::io::Result<usize> {
        let sec = read_from_partition(&self.partition, self.pos/512, buf.len().div_ceil(512).try_into().unwrap()).or(binrw::io::Result::Err(binrw::io::Error::new(binrw::io::ErrorKind::Other, 0)))?;
        let off = self.pos%512;
        for i in 0..buf.len() {
            buf[i] = sec[i+off as usize];
            self.pos += 1;
        }
        binrw::io::Result::Ok(buf.len())
    }
}
impl binrw::io::Seek for DiskReader {
    fn seek(&mut self, pos: binrw::io::SeekFrom) -> binrw::io::Result<u64> {
        self.pos = match pos {
            binrw::io::SeekFrom::Start(s) => {
                s
            },
            binrw::io::SeekFrom::End(e) => todo!(),
            binrw::io::SeekFrom::Current(c) => self.pos + <i64 as TryInto<u64>>::try_into(c).unwrap(),
        };
        binrw::io::Result::Ok(self.pos)
    }
}