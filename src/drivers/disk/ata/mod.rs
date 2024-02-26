use core::cell::OnceCell;
use core::sync::atomic::AtomicU8;
use core::{fmt::Display, panic};

use alloc::string::ToString;
use alloc::{format, vec::Vec};
use bit_field::BitField;
use log::{debug, error, info, trace};
use spin::{Mutex, MutexGuard, Once, RwLock};
use x86_64::instructions::port::{Port, PortReadAccess, PortWriteAccess};

use crate::bit_manipulation::{bytes, ptrlist_to_num, u16_to_u8};

#[cfg(feature = "fs")]
use crate::fs::partition::Partition;
#[cfg(feature = "fs")]
use crate::fs::path::FilePath;
use crate::pci::PciDevice;
use crate::x86_64::instructions::port::{PortRead, PortWrite};
use crate::{dbg, disk_manager};

use super::driver::{Disk, DiskDriver, DiskDriverEnum, DiskDriverType, GenericDisk, SECTOR_SIZE};
use super::{DiskError, DiskLoc};

pub mod driver;
pub mod irq;

pub static mut ATA_DRIVER: Option<RwLock<driver::AtaDriver>> = None;
pub static SELECTED_DISK: core::sync::atomic::AtomicU8 = AtomicU8::new(0);
/// Scans the different ATA disks
/// The buses are:
/// Primary Channel:   Slave & Master
/// Secondary Channel: Slave & Master
/// So the IDE controller only has a max of 4 drives
pub fn init(ide: &PciDevice) -> Vec<super::driver::Disk> {
    // // set bit 1 to disable interrupts
    // unsafe { u8::write_to_port(0x376, 1 << 2) }
    // unsafe { u8::write_to_port(0x3f6, 0) }
    // unsafe { u8::write_to_port(0x376, 0) }
    let raw_disks = [
        (
            detect(&DiskLoc(Channel::Primary, Drive::Master)),
            DiskLoc(Channel::Primary, Drive::Master),
        ),
        (
            detect(&DiskLoc(Channel::Primary, Drive::Slave)),
            DiskLoc(Channel::Primary, Drive::Slave),
        ),
        (
            detect(&DiskLoc(Channel::Secondary, Drive::Master)),
            DiskLoc(Channel::Secondary, Drive::Master),
        ),
        (
            detect(&DiskLoc(Channel::Secondary, Drive::Slave)),
            DiskLoc(Channel::Secondary, Drive::Slave),
        ),
    ];
    let mut disks = Vec::with_capacity(4);
    let mut gen_disks = Vec::with_capacity(4);
    for (disk, loc) in raw_disks {
        if let Some(disk) = &disk {
            gen_disks.push(Disk {
                loc,
                drv: DiskDriverEnum::Ata,
            });
        }
        disks.push(disk);
    }
    disks.shrink_to_fit();
    unsafe {
        ATA_DRIVER.replace(RwLock::new(driver::AtaDriver::new(
            disks.try_into().unwrap(),
        )));
    };
    gen_disks
}
pub enum DiskCommand {
    Reset = 0x90,
    ReadSectorsExt = 0x24,
    CacheFlush = 0xEA,
}
#[repr(u8)]
pub enum DiskRawErrorEnum {
    /// Address mark not found.
    AMNF,
    ///Track zero not found.
    TKZNF,
    /// Aborted command.
    ABRT,
    ///  Media change request.
    MCR,
    /// ID not found.
    IDNF,
    ///   Media changed.
    MC,
    ///  Uncorrectable data error.
    UNC,
    ///  Bad Block detected
    BBK,
}
#[repr(u8)]
pub enum DiskRawStatusEnum {
    ///  Indicates an error occurred. Send a new command to clear it (or nuke it with a Software Reset).
    ERR,
    ///  Index. Always set to zero.
    IDX,
    /// Corrected data. Always set to zero.
    CORR,
    ///  Set when the drive has PIO data to transfer, or is ready to accept PIO data.
    DRQ,
    ///  Overlapped Mode Service Request.
    SRV,
    ///   Drive Fault Error (does not set ERR).
    DF,
    ///  Bit is clear when drive is spun down, or after an error. Set otherwise.
    RDY,
    ///  Indicates the drive is preparing to send/receive data (wait for it to clear). In case of 'hang' (it never clears), do a software reset.
    BSY,
}

pub enum Reg {
    Data,
    Error,
    Features,
    SectorCount,
    LbaLo,
    LbaMi,
    LbaHi,
    DriveHead,
    Status,
    Command,
}
impl Reg {
    pub fn offset(&self) -> usize {
        match self {
            Reg::Data => 0,
            Reg::Error => 1,
            Reg::Features => 1,
            Reg::SectorCount => 2,
            Reg::LbaLo => 3,
            Reg::LbaMi => 4,
            Reg::LbaHi => 5,
            Reg::DriveHead => 6,
            Reg::Status => 7,
            Reg::Command => 7,
        }
    }
}

#[derive(Debug)]
pub struct AtaDisk {
    loc: DiskLoc,
    iobase: u16,
    control_base: u16,
    initialised: bool,
    drive_type: Option<DriveType>,
    addressing_modes: Option<(bool, u32, u64)>,
    is_hdd: Option<bool>,
}
impl AtaDisk {
    pub fn new(loc: DiskLoc, iobase: u16, control_base: u16) -> Self {
        Self {
            iobase,
            control_base,
            initialised: false,
            loc,
            drive_type: None,
            addressing_modes: None,
            is_hdd: None,
        }
    }
    pub fn size(&self) -> u64 {
        if let Some(addressing_modes) = self.addressing_modes {
            if addressing_modes.2 != 0 {
                addressing_modes.2
            } else if addressing_modes.1 != 0 {
                addressing_modes.1 as u64
            } else {
                1
            }
        } else {
            0
        }
    }
    /// A duplicate of the Status Register which does not affect interrupts
    /// You can use DiskRawStatusEnum with .get_bit
    pub fn alternate_status(&self) -> u8 {
        unsafe { u8::read_from_port(self.control_base) }
    }
    /// Used to reset the bus or enable/disable interrupts
    pub fn device_control(&self, command: u8) {
        unsafe { u8::write_to_port(self.control_base, command) }
    }
    // Tells the channel to select this drive
    pub fn select(&self) {
        unsafe { u8::write_to_port(self.iobase + 6, self.loc.drive_select_addr()) };
        //TODO Waiting ?
    }
    pub fn init(&mut self) -> Result<(), DiskError> {
        log::trace!("Initialising disk at {}", self.loc);
        unsafe { u8::write_to_port(self.control_base, 1 << 1) } // Disable interrupts for identify and disk selection
        self.select();
        self.identify()?;
        //TODO Interrupts & IRQ's
        unsafe { u8::write_to_port(self.control_base, 0) } // Enable interrupts

        self.initialised = true;
        Ok(())
    }
    // TODO Read and return error reg
    pub fn command(&self, command: DiskCommand) {
        unsafe { u8::write_to_port(self.iobase + 7, command as u8) }
    }
    //TODO u8 if lba28...
    //TODO Return a enum
    // Used to retrieve any error generated by the last ATA command executed.
    /// You can use DiskRawErrorEnum
    pub fn error(&self) -> u8 {
        unsafe { PortRead::read_from_port(self.iobase + 1) }
    }
    /// Used to control command specific interface features
    pub fn features(&self, features: u16) {
        unsafe { PortWrite::write_to_port(self.iobase + 1, features) }
    }
    pub fn drive_type(&self) -> DriveType {
        match self.drive_type {
            Some(r#type) => r#type,
            None => {
                //TODO Do proper waiting etc, working in qemu tho
                self.command(DiskCommand::Reset);
                let _sector_count = unsafe { u8::read_from_port(self.iobase + 2) };
                let _lba_low = unsafe { u8::read_from_port(self.iobase + 3) };
                let lba_mid = unsafe { u8::read_from_port(self.iobase + 4) };
                let lba_high = unsafe { u8::read_from_port(self.iobase + 5) };

                // let signature = (sector_count, lba_low, lba_mid, lba_high);
                let end_signature = (lba_mid, lba_high);

                match end_signature {
                    (0, 0) => DriveType::PATA,
                    (0x14, 0xEB) => DriveType::PATAPI,
                    (0x69, 0x96) => DriveType::SATAPI,
                    (0x3c, 0xc3) => DriveType::SATA,
                    _ => {
                        debug!("Found drive of unknown type: {:?}", end_signature);
                        DriveType::UNKNOWN
                    }
                }
            }
        }
    }
    pub fn identify(&mut self) -> Result<(), DiskError> {
        trace!(
            "Identifying drive: {:?} on channel: {:?} | Address: 0x{:X}",
            self.loc.drive(),
            self.loc.channel(),
            self.iobase
        );
        let drive_type = self.drive_type();
        let identify_command = if drive_type == DriveType::PATAPI {
            0xA1
        } else {
            0xECu8
        };
        self.write_reg(Reg::SectorCount, 0u8);
        self.write_reg(Reg::LbaLo, 0u8);
        self.write_reg(Reg::LbaMi, 0u8);
        self.write_reg(Reg::LbaHi, 0u8);

        unsafe {
            u8::write_to_port(self.iobase + 7, 0xE7);
            bsy(self.iobase);
        } //Clear cache
        self.write_reg(Reg::Command, identify_command);

        trace!("Reading drive status");
        if self.read_reg::<u8>(Reg::Status) == 0 {
            trace!("Drive does not exist !");
            return Err(DiskError::DiskNotFound);
        }
        unsafe {
            bsy(self.iobase);
        }
        if self.read_reg::<u8>(Reg::LbaMi) != 0 || self.read_reg::<u8>(Reg::LbaHi) != 0 {
            trace!("ATAPI drive detected !");
        } else if unsafe { check_drq_or_err(self.iobase) }.is_err() {
            error!(
                "Drive {:?} in {:?} channel returned an error after IDENTIFY command, please post an issue on github",
                self.loc.drive(),
                self.loc.channel()
            );
            return Err(DiskError::DiskNotFound);
        }
        let identify = read_identify(self.iobase);
        // core::ffi::CStr
        // info!("Serial number: {:?}\tFirmware revision: {:?}\tModel number: {:?}", &char_identify[20..40], &char_identify[46..52], &char_identify[54..92]);

        let lba28 = ptrlist_to_num(&mut identify[60..61].iter());
        let lba48: u64 = ptrlist_to_num(&mut identify[100..103].iter());
        let is_hardisk = true;
        //TODO Parse ALL info returned by IDENTIFY https://wiki.osdev.org/ATA_PIO_Mode
        // i.e. UDMA
        if lba28 as u64 + lba48 == 0 {
            // Skip if size = 0, because QEMU sometimes creates some disks with no size that isn't interesting
            return Err(DiskError::DiskNotFound);
        }
        trace!(
            "Found {:?} {:?} drive in {:?} channel of size: {}Ko",
            self.loc.drive(),
            drive_type,
            self.loc.channel(),
            ((lba48.max(lba28 as u64) * 512) / 1024)
        );
        let chs = false;
        // We set the values only if everything was ok
        self.addressing_modes = Some((chs, lba28, lba48));
        self.is_hdd = Some(is_hardisk);
        self.drive_type = Some(drive_type);

        Ok(())
    }
    //28Bit Lba PIO mode
    pub fn read28(&self, _lba: u32, sector_count: u8) -> Result<Vec<u8>, DiskError> {
        log::debug!("Reading 28, will it work ?");
        todo!();
        let drive_addr = self.loc.drive_lba28_addr() as u32;
        let base = self.loc.channel_addr();
        let lba28 = self.addressing_modes.ok_or(DiskError::Unitialised)?.1;
        unsafe {
            u8::write_to_port(
                base + 6,
                (drive_addr | ((lba28 >> 24) & 0x0F)).try_into().unwrap(),
            );
            u8::write_to_port(base + 1, 0x00);
            u8::write_to_port(base + 2, sector_count);
            u8::write_to_port(base + 3, (lba28 & 0xFF).try_into().unwrap());
            u8::write_to_port(base + 4, ((lba28 >> 8) & 0xFF).try_into().unwrap());
            u8::write_to_port(base + 5, ((lba28 >> 16) & 0xFF).try_into().unwrap());
            u8::write_to_port(base + 7, 0x20);
        }
        self.retrieve_read(sector_count.into())
    }
    /// Takes a &self so that read can also take & and not &mut
    fn write_reg<T: PortWrite>(&self, reg: Reg, value: T) {
        unsafe { T::write_to_port(self.iobase + reg.offset() as u16, value) }
    }
    fn read_reg<T: PortRead>(&self, reg: Reg) -> T {
        unsafe { T::read_from_port(self.iobase + reg.offset() as u16) }
    }
    // fn flush_cache(&self) {
    //     unsafe { u8::write_to_port(self.command_reg(), 0xE7) };
    //     unsafe { check_drq_or_err(self.base()) };
    // }
    //48Bit Lba PIO mode
    // 0 for sector_count is equals to u16::MAX
    pub fn read48(&self, lba: u64, sector_count: u16) -> Result<Vec<u8>, DiskError> {
        if lba + sector_count as u64 >= unsafe { self.addressing_modes.unwrap_unchecked().2 } {
            log::error!(
                "Trying to read sector outside of disk ! {lba}-{}",
                lba + sector_count as u64
            );
            return Err(DiskError::SectorTooBig);
        }
        self.write_reg(Reg::DriveHead, self.loc.drive_lba48_addr());
        self.write_reg(Reg::Data, self.read_reg::<u8>(Reg::Data) | 0x80);

        self.write_reg(Reg::SectorCount, (sector_count >> 8) as u8); // sector_count high
        self.write_reg(Reg::LbaLo, (lba >> 24) as u8); // LBA4
        self.write_reg(Reg::LbaMi, (lba >> 32) as u8); // LBA5
        self.write_reg(Reg::LbaHi, (lba >> 40) as u8); // LBA6

        self.write_reg(Reg::Data, self.read_reg::<u8>(Reg::Data) & !0x80);

        self.write_reg(Reg::SectorCount, sector_count as u8); // sector_count low
        self.write_reg(Reg::LbaLo, lba as u8); // LBA1
        self.write_reg(Reg::LbaMi, (lba >> 8) as u8); // LBA2
        self.write_reg(Reg::LbaHi, (lba >> 16) as u8); // LBA3
        self.command(DiskCommand::ReadSectorsExt); // READ SECTORS EXT

        self.retrieve_read(sector_count)
    }
    fn retrieve_read(&self, sector_count: u16) -> Result<Vec<u8>, DiskError> {
        trace!("Retrieving read !");
        let mut buffer = Vec::with_capacity(sector_count as usize * 512);
        for _sector in 0..sector_count {
            self.poll()?;
            for i in 0..SECTOR_SIZE / 4 {
                // Divide by 4 because we take 4 by 4 bytes
                let data = self.read_reg::<u32>(Reg::Data);
                // Do we make a for loop ?
                buffer.push(data as u8); // Try push_within_capacity
                buffer.push((data >> 8) as u8);
                buffer.push((data >> 16) as u8);
                buffer.push((data >> 24) as u8);
            }
        }
        Ok(buffer)
    }

    pub fn write48(&self, start_sector: u64, content: &[u8]) -> Result<(), DiskError> {
        todo!()
    }
    //     let mut sector_count = content.len().div_ceil(512);
    //     debug!("{} {:?}", start_sector, content);
    //     self.write_reg(Reg::DriveHead, self.loc.drive_lba48_addr());
    //     self.write_reg(Reg::Data, self.read_reg::<u8>(Reg::Data) | 0x80);

    //         self.write_reg(Reg::SectorCount, TryInto::<u8>::try_into(sector_count >> 8).or(DiskError::SectorTooBig)?); // sector_count high
    //         self.write_reg(Reg::LbaLo, TryInto::<u8>::try_into(start_sector >> 24).or(DiskError::SectorTooBig)?,
    //         ); // LBA4
    //         self.write_reg(Reg::
    //             self.lbamid_reg(),
    //             TryInto::<u8>::try_into(start_sector >> 32).or(DiskError::SectorTooBig)?,
    //         ); // LBA5
    //         self.write_reg(Reg::
    //             self.lbahi_reg(),
    //             TryInto::<u8>::try_into(start_sector >> 40).or(DiskError::SectorTooBig)?,
    //         ); // LBA6

    //         self.write_reg(Reg::self.base(), u8::read_from_port(self.base()) & !0x80);

    //         self.write_reg(Reg::
    //             self.sector_count_reg(),
    //             TryInto::<u8>::try_into(sector_count).or(DiskError::SectorTooBig)?,
    //         ); // sector_count low
    //         self.write_reg(Reg::
    //             self.lbalo_reg(),
    //             TryInto::<u8>::try_into(start_sector & 0xFF).or(DiskError::SectorTooBig)?,
    //         ); // LBA1
    //         self.write_reg(Reg::
    //             self.lbamid_reg(),
    //             TryInto::<u8>::try_into(start_sector >> 8).or(DiskError::SectorTooBig)?,
    //         ); // LBA2
    //         self.write_reg(Reg::
    //             self.lbahi_reg(),
    //             TryInto::<u8>::try_into(start_sector >> 16).or(DiskError::SectorTooBig)?,
    //         ); // LBA3
    //         self.write_reg(Reg::self.command_reg(), 0x34); // READ SECTORS EXT
    //     }
    //     self.send_write(content)
    // }
    fn send_write(&self, content: &[u8]) -> Result<(), DiskError> {
        let mut len = content.len() / 512;
        if len == 0 {
            len += 1
        }
        for sector in 0..len {
            self.poll()?;

            for i in 0..128 {
                let mut data = 0;
                for j in 0..4 {
                    data |=
                        (*content.get((sector * 512) + i * 4 + j).unwrap_or(&0) as u32) << (8 * j)
                }
                self.write_reg(Reg::Data, data);
            }
        }
        // Cache flush
        self.command(DiskCommand::CacheFlush);
        self.poll()?;
        Ok(())
    }

    /*
    #define ATA_SR_BSY     0x80    // Busy
    #define ATA_SR_DRDY    0x40    // Drive ready
    #define ATA_SR_DF      0x20    // Drive write fault
    #define ATA_SR_DSC     0x10    // Drive seek complete
    #define ATA_SR_DRQ     0x08    // Data request ready
    #define ATA_SR_CORR    0x04    // Corrected data
    #define ATA_SR_IDX     0x02    // Index
    #define ATA_SR_ERR     0x01    // Error
    */
    fn poll(&self) -> Result<(), DiskError> {
        for _ in 0..4 {
            // Doing this 4 times creates a 400ns delay
            let _ = self.read_reg::<u8>(Reg::Data);
        }
        for i in 0..100_000 {
            let status = self.check_status()?;
            if status & 0x80 == 0 {
                if status & 0x08 == 0x08 {
                    return Ok(()); // Read data available
                } else if status & 0x01 == 0x1 {
                    // Error reading
                    error!("Error reading disk !");
                    return Err(DiskError::ReadDataNotAvailable);
                } else if status & 0x20 == 0x20 {
                    // Error reading
                    error!("Error reading disk !");
                    return Err(DiskError::ReadDataNotAvailable);
                }
            }
            if i == 100_000 - 1 {
                log::error!("DRQ read timed out, polling with status 0x{:02X}", status);
                return Err(DiskError::ReadDataNotAvailable);
            }
        }
        Ok(())
    }
    fn check_status(&self) -> Result<u8, DiskError> {
        let status = self.read_reg(Reg::Status);

        if status & 0x01 != 0 {
            log::error!("IDE error: {:#x}", unsafe { self.error() });
            return Err(DiskError::DRQRead);
        }

        if status & 0x20 != 0 {
            log::error!("IDE device write fault");
            return Err(DiskError::DRQRead);
        }

        Ok(status)
    }

    pub fn read_sectors(
        &self,
        sector_address: u64,
        sector_count: u16,
    ) -> Result<Vec<u8>, DiskError> {
        //TODO Move from vecs to slices, to have same way of functionning than NVMe for example
        let addressing_modes = self.addressing_modes.ok_or(DiskError::Unitialised)?;
        if addressing_modes.2 != 0 {
            if sector_address + (sector_count as u64) > addressing_modes.2 {
                // > or >= ?
                error!(
                    "Sector not in disk ({} - {}) -> {:?}",
                    sector_address, sector_count, self.loc
                );
                return Err(DiskError::NotFound);
            }
            self.read48(sector_address, sector_count)
        } else if addressing_modes.1 != 0 {
            let sector_address = sector_address.try_into().or(Err(DiskError::SectorTooBig))?;
            let sector_count = sector_count.try_into().or(Err(DiskError::SectorTooBig))?;
            self.read28(sector_address, sector_count)
        } else if addressing_modes.0 {
            log::error!("Implement CHS pio mode");
            return Err(DiskError::NoReadModeAvailable);
            // return self.readchs(sector_address.try_into()?, sector_count.try_into()?)
        } else {
            Err(DiskError::NoReadModeAvailable)
        }
    }
    pub fn write_sectors(&self, start_sector: u64, content: &[u8]) -> Result<(), DiskError> {
        let addressing_modes = self.addressing_modes.ok_or(DiskError::Unitialised)?;
        if addressing_modes.2 != 0 {
            self.write48(start_sector, content)
        } else if addressing_modes.1 != 0 {
            todo!("Implement lba28 mode");
            // self.write28(start_sector, content)
        } else if addressing_modes.0 {
            todo!("Implement CHS pio mode");
        } else {
            Err(DiskError::NoReadModeAvailable)
        }
    }
}

impl Display for AtaDisk {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&format!("Disk: {}Ko on {:?}", self.size() / 1024, self.loc))?;
        Ok(())
    }
}
impl GenericDisk for AtaDisk {
    fn loc(&self) -> &DiskLoc {
        &self.loc
    }
}
/// Detects a disk at specified channel and drive
/// Reads identify data & sets it up correctly
fn detect(loc: &DiskLoc) -> Option<AtaDisk> {
    let control_base = match loc.channel() {
        //TODO Parse pci device to get info
        Channel::Primary => 0x3F6,
        Channel::Secondary => 0x376,
    };
    let mut disk = AtaDisk::new(*loc, loc.base(), control_base);
    disk.init().ok()?;
    Some(disk)
}

fn read_identify(command_port_addr: u16) -> [u16; SECTOR_SIZE as usize / 2] {
    trace!("Reading identify data");
    let mut data = [0u16; 256];
    for ele in &mut data {
        *ele = unsafe { u16::read_from_port(command_port_addr) };
    }
    data
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Channel {
    Primary = 0x1F0,
    Secondary = 0x170,
}
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum Drive {
    Master,
    Slave,
}

#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub enum DriveType {
    PATA,
    PATAPI,
    SATAPI,
    SATA,
    UNKNOWN,
}

#[derive(Debug)]
pub struct AddressingModes {
    chs: bool,
    lba28: u32, // total number of 28 bit LBA addressable sectors on the drive. (If non-zero, the drive supports LBA28.)
    lba48: u64,
}
/// wait for BSY flag to be unset
unsafe fn bsy(base: u16) {
    trace!("Waiting BSY flag to unset at base: {:X}", base);
    while unsafe { u8::read_from_port(base + 7) } & 0x80 != 0x00 {}
    // 0x80 = 0b10000000
}

/// wait for DRQ to be ready or ERR to set
unsafe fn check_drq_or_err(base: u16) -> Result<(), DiskError> {
    trace!("Waiting DRQ flag to set at base: {:X}", base);
    let mut status = unsafe { u8::read_from_port(base + 7) };
    let mut i = 0;
    loop {
        if status.get_bit(0) {
            error!(
                "Error reading DRQ from drive: {}",
                bytes(unsafe { u8::read_from_port(base + 1) })
            );
            return Err(DiskError::DRQRead);
        } //TODO Make better error handling... Or make error handling in top level function
        if status.get_bit(4) {
            break;
        }
        if i > 10000000 {
            error!("Error reading DRQ from drive: TIMEOUT"); // TODO Timeout with PIT
            return Err(DiskError::TimeOut);
        }
        status = unsafe { u8::read_from_port(base + 7) };
        i += 1;
    }
    Ok(())
}
