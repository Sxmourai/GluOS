use alloc::{
    format,
    string::{String, ToString},
};

#[derive(Default, Clone)]
#[repr(C, packed)]
pub struct Standard32 {
    // r:u8,
    pub name: [u8; 8],
    pub extension: [u8; 3],
    pub attributes: u8,
    pub reserved: u8,
    pub duration_creation_time: u8,
    pub creation_time: u16,
    pub creation_date: u16,
    pub last_accessed_date: u16,
    pub high_u16_1st_cluster: u16,
    pub last_modif_time: u16,
    pub last_modif_date: u16,
    pub low_u16_1st_cluster: u16,
    pub size: u32,
}
impl Standard32 {
    pub fn name(&self) -> String {
        String::from_utf8_lossy(
            [self.name.to_vec(), self.extension.to_vec()]
                .concat()
                .as_slice(),
        )
        .to_string()
    }
    pub fn printable(&self, first_data_sector: u64) -> String {
        let creation_date = self.creation_date;
        let fst_cluster =
            ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let sector = if fst_cluster > 2 {
            (fst_cluster as u64 - 2) + first_data_sector
        } else {
            0
        };
        let size = self.size;
        let _entry_type = self.name[0];
        let name = self.name();
        format!(
            "File8.3: {}\t | creation_date: {} | 1st cluster: {}({}) \t| size: {}\t| attrs: {:#b}",
            name, creation_date, fst_cluster, sector, size, self.attributes
        )
    }
}
impl core::fmt::Debug for Standard32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let creation_date = self.creation_date;
        let fst_cluster =
            ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let size = self.size;
        let _entry_type = self.name[0];
        let _name = self.name();
        f.write_str(format!("Standard8.3Entry({} creation_date: {} | 1st cluster: {} \t| size: {}\t| attrs: {:#b})", self.name(), creation_date, fst_cluster, size, self.attributes).as_str())
    }
}

#[derive(Clone)]
#[repr(C, packed)]
pub struct LFN32 {
    order: u8, // The order of this entry in the sequence of long file name entries. This value helps you to know where in the file's name the characters from this entry should be placed.
    fst_chars: [u16; 5],
    attribute: u8,       // Should ALWAYS be 0xF
    long_entry_type: u8, // Zero for name entries
    chksum: u8, // Checksum generated of the short file name when the file was created. The short filename can change without changing the long filename in cases where the partition is mounted on a system which does not support long filenames
    scd_chars: [u16; 6],
    zeroes: u16,
    fin_chars: [u16; 2],
}

impl LFN32 {
    pub fn name(&self) -> String {
        let fst_chars = self.fst_chars; // To get packed fields
        let scd_chars = self.scd_chars;
        let fin_chars = self.fin_chars;
        let raw_name = [fst_chars.to_vec(), scd_chars.to_vec(), fin_chars.to_vec()].concat();

        let mut name = String::new();
        for chr in raw_name {
            if chr == 0 || chr == 255 {
                continue;
            }
            name.push_str(String::from_utf16_lossy(&[chr]).to_string().as_str());
        }
        name
        // String::from_utf16_lossy(&raw_name).to_string()
    }
}
impl core::fmt::Debug for LFN32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(format!("LFN32({})", self.name()).as_str())
    }
}
#[derive(Debug, Clone)]
pub enum RawFat32Entry {
    LFN(LFN32),
    Standard(Standard32),
}
