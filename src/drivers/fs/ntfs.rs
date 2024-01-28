
use alloc::{string::{String, ToString}, vec::Vec};

use crate::{bit_manipulation::all_zeroes, dbg, disk::ata::read_from_partition, serial_println};

use super::{fs_driver::{FsDriver, FsDriverInitialiser}, partition::Partition};

pub const NTFS_MAGIC: [u8; 8] = [78, 84, 70, 83, 32, 32, 32, 32];

#[derive(Debug)]
pub struct NTFSDriver {
    partition: Partition,
}

impl FsDriverInitialiser for NTFSDriver {
    fn try_init(partition: &Partition) -> Option<alloc::boxed::Box<Self>> where Self: Sized {
        let raw_ntfs_table = read_from_partition(partition, 0, 1).ok()?;
        let ntfs_table = unsafe {&*(raw_ntfs_table.as_ptr() as *const NTFSTable )};
        if ntfs_table.oem_name!=NTFS_MAGIC{return None}
        dbg!(ntfs_table);
        read_mft(partition, ntfs_table.master_file_table_cluster, ntfs_table.sectors_per_cluster as u64);
        Some(alloc::boxed::Box::new(Self {
            partition: partition.clone(),
        }))
    }
}
impl FsDriver for NTFSDriver {
    fn read(&self, path: &super::fs::FilePath) -> Result<super::fs_driver::Entry, super::fs_driver::FsReadError> {
        todo!()
    }

    fn as_enum(&self) -> super::fs_driver::FsDriverEnum {
        super::fs_driver::FsDriverEnum::NTFS
    }

    fn partition(&self) -> &super::partition::Partition {
        &self.partition
    }
}

fn read_mft(partition: &Partition, master_file_table_cluster: u64, sectors_per_cluster: u64) -> Option<()> {
    let mut record_idx = 1;
    'record: loop {
        //TODO Read right amount of sectors, but in my case clusters_per_record=246  and  sectors_per_cluster=8 so 246*8*512=1007616 which will take to much heap and time
        let raw_record = read_from_partition(partition, record_idx*master_file_table_cluster*sectors_per_cluster as u64, 40).ok()?;
        if all_zeroes(&raw_record){break}
        let record = unsafe{&*(raw_record.as_ptr() as *const MasterFileTableRecord)};
        if record.flags&0x2==0x2 {
            log::info!("DIRECTORY = VICTORY !");
        }
        let attributes = &raw_record[record.attrs_offset as usize..];
        let mut attr_offset = 0;
        let attrs = parse_attributes(attributes);
        
        record_idx += 1;
    }
    Some(())
}

fn parse_attributes(attrs: &[u8]) -> Vec<MFTAttribute> {
    let parsed_attrs = Vec::new();
    let mut attr_offset = 0;
    loop {
        let attr_type = ((attrs[attr_offset+3] as u32) << 24) | ((attrs[attr_offset+2] as u32) << 16) | ((attrs[attr_offset+1] as u32) << 8) | attrs[attr_offset+0] as u32;
        if attr_type==0xffffffff {
            attr_offset += 4;
            break
        }
        if attr_type==0 {
            log::error!("Error reading attribute of 0?! {:?}", (attrs));
            break;
        }
        let attr = unsafe {&*(attrs[attr_offset..].as_ptr() as *const MFTAttribute)};
        if attr.resident_flag==0 { // Resident
            let res_data = unsafe {&*(attrs[attr_offset+16..].as_ptr() as *const ResidentData)};
            dbg!(res_data);
        }
        else if attr.resident_flag==1 { // Non resident
            let nonres_data = unsafe {&*(attrs[attr_offset+16..].as_ptr() as *const NonResidentData)};
            
            dbg!(nonres_data);
        }
        
        if attr.name_len>0 {
            todo!()
        }
        match attr_type/16 {
            3 => {
                let raw_name = &attrs[attr_offset+90..attr_offset+attr.len as usize];
                let mut name = String::new();
                for b2 in raw_name.chunks_exact(2) {
                    let word = (b2[1] as u16)<<8 | b2[0] as u16;
                    name.push_str(String::from_utf16_lossy(&[word, ]).as_str());
                }
                dbg!(name);
            }
            _ => {}
        }
        serial_println!();
        attr_offset += attr.len as usize;
    }
    parsed_attrs
}


#[derive(Debug)]
#[repr(packed)]
pub struct NTFSTable {
    pub bootjmp: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sectors: u16,
    pub table_count: u8,
    pub root_entries: u16,
    pub sector_count: u16,
    pub media_type: u8,
    pub sectors_per_table: u16,
    pub sectors_per_track: u16,
    pub heads: u16,
    pub hidden_sectors_count: u32,
    pub sector_count_32: u32,
    pub reserved: u32,
    pub sector_count_64: u64,

    // NTFS Specific header
    master_file_table_cluster: u64,
    master_file_table_mirror_cluster: u64,
    clusters_per_record: u8,
    reserved_: [u8; 3],
    clusters_per_index_buffer: u8,
    reserved__: [u8; 3],
    serial_number: u64,
    checksum: u32,
}

#[derive(Debug)]
#[repr(packed)]
struct MasterFileTableRecord {
    record_type: [u8; 4],
    update_seq_offset: u16,
    update_seq_len: u16,
    log_file_seq_num: u64,
    record_seq_num: u16,
    hard_link_count: u16,
    attrs_offset: u16,
    flags: u16,
    bytes_in_use: u32,
    bytes_allocated: u32,
    parent_record_number: u64,
    next_attr_index: u32,
    reserved: u32,
    record_num: u64,
}
#[repr(C, packed)]
#[derive(Debug)]
struct MFTAttribute {
    attr_type: u32,
    len: u32,
    /// 0=Resident | 1 =Nonresident
    resident_flag: u8, 
    name_len: u8, // Contains the number of characters without the end-of-string character
    name_offset: u16,
    attr_data_flag: u16,
    id: u16,
}
#[repr(C, packed)]
#[derive(Debug)]
struct ResidentData {
    val_len: u32,
    val_offset: u16,
    /// Only the lower bit is used, do the other bits have any significance? Check https://github.com/libyal/libfsntfs/blob/main/documentation/New%20Technologies%20File%20System%20(NTFS).asciidoc#attribute_chains
    indexed_flag: u8, 
    _padding: u8,
}
#[repr(C, packed)]
#[derive(Debug)]
struct NonResidentData {
    first_vcn: u64,
    /// Seen this value to be -1 in combination with data size of 0
    last_vcn: u64,
    /// Contains an offset relative from the start of the MFT attribute
    ///! Note: The total size of the data runs should be larger or equal to the data size.
    data_runs_offset: u16, // or mappings pairs offset
    //TODO Compression ?
    compression_unit_size: u16,
    _padding: u32,
    ///Contains the allocated data size in number of bytes.
    ///This value is not valid if the first VCN is nonzero.
    allocated_data_size: u64,
    ///Contains the data size in number of bytes.
    ///This value is not valid if the first VCN is nonzero.
    file_size: u64,
    ///Contains the valid data size in number of bytes. 
    ///This value is not valid if the first VCN is nonzero.
    valid_data_size: u64,

    /// If comp unit >0
    /// Contains the total allocated size in number of cluster blocks
    total_allocated_size: u64,
}


#[repr(C, packed)]
#[derive(Debug)]
struct DataRuns {
    ///Low: Number of cluster blocks value size: Contains the number of bytes used to store the data run size
    ///Hig: Cluster block number value size    : Contains the number of bytes used to store the data run size
    number_cluster_blocks_value_size_and_cluster_block_number_value_size: u8,
    /// Data run number of cluster blocks
    /// Contains the number of cluster blocks
    size_value_size: u8,
    
}




const FILE_RECORD_MFT_MAGIC: [u8; 4] = [70, 73, 76, 69];//['F' as u8, 'I' as u8, 'L' as u8, 'E' as u8];

static MFT_ATTRIBUTES: &[&str] = &[
    "Unknown",
    "Standard infos",
    "Attribute list",
    "File name",
    "Object ID",
    "Security descriptor",
    "Volume name",
    "Volume infos",
    "Data",
    "Index root",
    "Index allocation",
    "Bitmap",
    "Reparse Point",
    "Unknown",
    "Unknown",
    "Unknown",
    "Logged Tool stream",
];