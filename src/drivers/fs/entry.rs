use alloc::{string::{String, ToString}, format};

#[derive(Debug, Default)]
#[repr(packed)]
pub struct Dir83Format {
    // r:u8,
    pub name: [u8; 11],
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
    pub size: u32
}
impl Dir83Format {
    pub fn lfn_name(raw_self: &[u8]) -> String {
        let mut name = String::new();
        let mut raw_name = raw_self[1..11].to_vec();
        raw_name.extend_from_slice(&raw_self[14..=26]);
        raw_name.extend_from_slice(&raw_self[28..31]);
        for chunk in raw_name.chunks_exact(2) {
            let chr = u16::from_ne_bytes([chunk[0], chunk[1]]);
            if chr == 0 {break}
            name.push_str(String::from_utf16_lossy(&[chr]).as_str());
        }
        name
    }
    pub fn name(&self) -> String {
        String::from_utf8_lossy(&self.name).to_string()
    }
    pub fn printable(&self, first_data_sector:u64) -> String {
        let creation_date = self.creation_date;
        let fst_cluster = ((self.high_u16_1st_cluster as u32) << 16) | (self.low_u16_1st_cluster as u32);
        let sector  = if fst_cluster>2 {
            (fst_cluster as u64 - 2)+first_data_sector
        } else { 0 };
        let size = self.size;
        let entry_type = self.name[0];
        let name = self.name();
        format!("File8.3: {}\t | creation_date: {} | 1st cluster: {}({}) \t| size: {}\t| attrs: {:#b}", name, creation_date, fst_cluster,sector, size, self.attributes)
    }
}