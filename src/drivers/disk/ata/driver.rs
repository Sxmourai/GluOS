use super::*;

#[derive(Debug)]
pub struct AtaDriver {
    selected_disk: u8,
    disks: [Option<AtaDisk>; 4],
}
impl AtaDriver {
    pub fn new(disks: [Option<AtaDisk>; 4]) -> Self {
        Self {
            selected_disk: 0,
            disks,
        }
    }
}
impl super::DiskDriver for AtaDriver {
    fn read(
        &mut self,
        loc: &DiskLoc,
        start_sector: u64,
        sector_count: u64,
    ) -> Result<Vec<u8>, DiskError> {
        self.select_disk(loc);
        self.disks[loc.as_index()]
            .as_mut()
            .ok_or(DiskError::NotFound)?
            .read_sectors(start_sector, sector_count.try_into().unwrap())
    }

    fn write(&mut self, loc: &DiskLoc, start_sector: u64, content: &[u8]) -> Result<(), DiskError> {
        todo!()
    }

    fn select_disk(&mut self, loc: &DiskLoc) {
        if loc.as_index() == self.selected_disk as usize {
            return;
        }
        let mut disk = &mut self.disks[loc.as_index()];
        disk.as_mut().unwrap().select();
        self.selected_disk = loc.as_index().try_into().unwrap();
        SELECTED_DISK.store(self.selected_disk, core::sync::atomic::Ordering::Release)
    }
}
impl AtaDriver {
    pub fn selected_disk(&self) -> DiskLoc {
        DiskLoc::from_idx(self.selected_disk).unwrap()
    }
}
