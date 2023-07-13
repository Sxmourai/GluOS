/// Implementation Courtesy of MOROS.
/// Currently Only Supports ATA-PIO, with 24-bit LBA Addressing.

/// COMES FROM https://docs.rs/crate/ata_x86/0.1.1/source/src/lib.rs

// extern crate alloc;

// use alloc::string::String;
// use alloc::vec::Vec;
// use bit_field::BitField;
// use core::{hint::spin_loop, arch::asm};
// use lazy_static::lazy_static;
// use spin::Mutex;
// // use x86_64::instructions::port::{Port, PortReadOnly, PortWriteOnly};
// use super::port::{Port, PortReadOnly, PortWriteOnly};

// use crate::serial_println;


// pub type BlockIndex = u32;

// pub const ATA_BLOCK_SIZE: usize = 512;

// fn sleep_ticks(ticks: usize) {
//     for _ in 0..=ticks {
//         x86_64::instructions::hlt();
//     }
// }

// #[repr(u16)]
// enum Command {
//     Read = 0x20,
//     Write = 0x30,
//     Identify = 0xEC,
// }

// #[allow(dead_code)]
// #[repr(usize)]
// enum Status {
//     ERR = 0,
//     IDX = 1,
//     CORR = 2,
//     DRQ = 3,
//     SRV = 4,
//     DF = 5,
//     RDY = 6,
//     BSY = 7,
// }

// #[allow(dead_code)]
// #[derive(Debug, Clone)]
// pub struct Bus {
//     id: u8,
//     irq: u8,

//     data_register: Port<u16>,
//     error_register: PortReadOnly<u8>,
//     features_register: PortWriteOnly<u8>,
//     sector_count_register: Port<u8>,
//         lba0_register: Port<u8>,
//     lba1_register: Port<u8>,
//     lba2_register: Port<u8>,
//     drive_register: Port<u8>,
//     status_register: PortReadOnly<u8>,
//     command_register: PortWriteOnly<u8>,

//     alternate_status_register: PortReadOnly<u8>,
//     control_register: PortWriteOnly<u8>,
//     drive_blockess_register: PortReadOnly<u8>,
// }

// impl Bus {
//     pub fn new(id: u8, io_base: u16, ctrl_base: u16, irq: u8) -> Self {
//         Self {
//             id, irq,

//             data_register: Port::new(io_base + 0),
//             error_register: PortReadOnly::new(io_base + 1),
//             features_register: PortWriteOnly::new(io_base + 1),
//             sector_count_register: Port::new(io_base + 2),
//             lba0_register: Port::new(io_base + 3),
//             lba1_register: Port::new(io_base + 4),
//             lba2_register: Port::new(io_base + 5),
//             drive_register: Port::new(io_base + 6),
//             status_register: PortReadOnly::new(io_base + 7),
//             command_register: PortWriteOnly::new(io_base + 7),

//             alternate_status_register: PortReadOnly::new(ctrl_base + 0),
//             control_register: PortWriteOnly::new(ctrl_base + 0),
//             drive_blockess_register: PortReadOnly::new(ctrl_base + 1),
//         }
//     }

//     fn reset(&mut self) {
//         unsafe {
//             self.control_register.write(4); // Set SRST bit
//             sleep_ticks(2);
//             self.control_register.write(0); // Then clear it
//             sleep_ticks(2);
//         }
//     }

//     fn wait(&mut self) {
//         for _ in 0..4 { // Wait about 4 x 100 ns
//             unsafe { self.alternate_status_register.read(); }
//         }
//     }

//     fn write_command(&mut self, cmd: Command) {
//         unsafe {
//             self.command_register.write(cmd as u8);
//         }
//     }

//     fn status(&mut self) -> u8 {
//         unsafe { self.status_register.read() }
//     }

//     fn lba1(&mut self) -> u8 {
//         unsafe { self.lba1_register.read() }
//     }

//     fn lba2(&mut self) -> u8 {
//         unsafe { self.lba2_register.read() }
//     }

//     fn read_data(&mut self) -> u16 {
//         unsafe { self.data_register.read() }
//     }

//     fn write_data(&mut self, data: u16) {
//         unsafe { self.data_register.write(data) }
//     }

//     fn busy_loop(&mut self) {
//         self.wait();
//         let start = 0;
//         while self.is_busy() {
//             if 0 - start > 1 { // Hanged
//                 return self.reset();
//             }

//             spin_loop();
//         }
//     }

//     fn is_busy(&mut self) -> bool {
//         self.status().get_bit(Status::BSY as usize)
//     }

//     fn is_error(&mut self) -> bool {
//         self.status().get_bit(Status::ERR as usize)
//     }

//     fn is_ready(&mut self) -> bool {
//         self.status().get_bit(Status::RDY as usize)
//     }

//     fn select_drive(&mut self, drive: u8) {
//         // Drive #0 (primary) = 0xA0
//         // Drive #1 (secondary) = 0xB0
//         let drive_id = 0xA0 | (drive << 4);
//         unsafe {
//             self.drive_register.write(drive_id);
//         }
//     }

//     fn setup(&mut self, drive: u8, block: u32) {
//         let drive_id = 0xE0 | (drive << 4);
//         unsafe {
//             self.drive_register.write(drive_id | ((block.get_bits(24..28) as u8) & 0x0F));
//             self.sector_count_register.write(1);
//             self.lba0_register.write(block.get_bits(0..8) as u8);
//             self.lba1_register.write(block.get_bits(8..16) as u8);
//             self.lba2_register.write(block.get_bits(16..24) as u8);
//         }
//     }

//     pub fn identify_drive(&mut self, drive: u8) -> Option<[u16; 256]> {
//         self.reset();serial_println!("R");
//         self.wait();serial_println!("W");
//         self.select_drive(drive);serial_println!("SELEC");
//         unsafe {
//             self.sector_count_register.write(0);serial_println!("WRITE SECTOR");
//             self.lba0_register.write(0);serial_println!("W LBA0");
//             self.lba1_register.write(0);serial_println!("W lba1");
//             self.lba2_register.write(0);serial_println!("W lba2");
//         }

//         self.write_command(Command::Identify);serial_println!("Identifz cmd");

//         if self.status() == 0 { 
//             return None;
//         }
//         serial_println!("STATUD");
//         self.busy_loop(); serial_println!("BUSY LOOP");

//         if self.lba1() != 0 || self.lba2() != 0 {
//             return None;
//         }serial_println!("lba1 == 0 && lba2 == 0");

//         for i in 0.. {
//             serial_println!("{}", i);
//             if i == 256 { 
//                 self.reset();
//                 return None;
//             }
//             if self.is_error() {
//                 return None;
//             }
//             if self.is_ready() {
//                 break;
//             }
//         }

//         let mut res = [0; 256];
//         for i in 0..256 {
//             res[i] = self.read_data();
//         }
//         Some(res)
//     }

//     /// Read A single, 512-byte long slice from a given block
//     /// panics if buf isn't EXACTLY 512 Bytes long;
//     /// Example:
//     /// ```rust
//     /// // Read A Single block from a disk
//     /// pub fn read_single() {
//     ///     use x86_ata::{init, ATA_BLOCK_SIZE, read};
//     ///     // 1. Initialise ATA Subsystem. (Perform Once, on boot)
//     ///     init().expect("Failed To Start ATA...");  
//     ///     // 2. Create a temporary buffer of size 512.
//     ///     let mut buffer: [u8;ATA_BLOCK_SIZE] = [0; ATA_BLOCK_SIZE];
//     ///     // 3. Pass the buffer over to the Subsystem, to be filled.
//     ///     read(0, 0, 0, &mut buffer);
//     /// }

//     pub fn read(&mut self, drive: u8, block: BlockIndex, buf: &mut [u8]) {
//         assert!(buf.len() == 512);
//         //log!("Reading Block 0x{:8X}\n", block);
//         //log!("{:?}", self);

//         self.setup(drive, block);
//         self.write_command(Command::Read);
//         self.busy_loop();
//         for i in (0..256).step_by(2) {
//             let data = self.read_data();

//             //log!("Read[{:08X}][{:02X}]: 0x{:04X}\n", block, i, data);
//             buf[i + 0] = data.get_bits(0..8) as u8;
//             buf[i + 1] = data.get_bits(8..16) as u8;
//         }
//     }

//     /// Write A single, 512-byte long slice to a given block
//     /// panics if buf isn't EXACTLY 512 Bytes long;
//     /// Example:
//     /// ```rust
//     /// // Read A Single block from a disk
//     /// pub fn write_single() {
//     ///     use x86_ata::{init, ATA_BLOCK_SIZE, write};
//     ///     // 1. Initialise ATA Subsystem. (Perform Once, on boot)
//     ///     init().expect("Failed To Start ATA...");  
//     ///     // 2. Create a temporary buffer of size 512.
//     ///     let buffer: [u8;ATA_BLOCK_SIZE] = [0; ATA_BLOCK_SIZE];
//     ///     // 3. Pass the buffer over to the Subsystem, to be filled.
//     ///     write(0, 0, 0, &buffer);
//     /// }

//     pub fn write(&mut self, drive: u8, block: BlockIndex, buf: &[u8]) {
//         assert!(buf.len() == 512);
//         self.setup(drive, block);
//         self.write_command(Command::Write);
//         self.busy_loop();
//         for i in 0..256 {
//             let mut data = 0 as u16;
//             data.set_bits(0..8, buf[i * 2] as u16);
//             data.set_bits(8..16, buf[i * 2 + 1] as u16);

//             //log!("Data: 0x{:04X} | {}{}    \n", data, buf[i * 2] as char, buf[i * 2 + 1] as char);

//             self.write_data(data);
//         }
//         self.busy_loop();
//     }
// }

// lazy_static! {
//     pub static ref BUSES: Mutex<Vec<Bus>> = Mutex::new(Vec::new());
// }

// fn disk_size(sectors: u32) -> (u32, String) {
//     let bytes = sectors * 512;
//     if bytes >> 20 < 1000 {
//         (bytes >> 20, String::from("MB"))
//     } else {
//         (bytes >> 30, String::from("GB"))
//     }
// }



// pub fn list() -> Vec<(u8, u8, String, String, u32, String, u32)> {
//     let mut buses = BUSES.lock();
//     let mut res = Vec::new();
//     for bus in 0..2 {
//         for drive in 0..2 {
//             if let Some(buf) = buses[bus as usize].identify_drive(drive) {
//                 let mut serial = String::new();
//                 for i in 10..20 {
//                     for &b in &buf[i].to_be_bytes() {
//                         serial.push(b as char);
//                     }
//                 }
//                 serial = serial.trim().into();
//                 let mut model = String::new();
//                 for i in 27..47 {
//                     for &b in &buf[i].to_be_bytes() {
//                         model.push(b as char);
//                     }
//                 }
//                 model = model.trim().into();
//                 let sectors = (buf[61] as u32) << 16 | (buf[60] as u32);
//                 let (size, unit) = disk_size(sectors);
//                 res.push((bus, drive, model, serial, size, unit, sectors));
//             }
//         }
//     }
//     res
// }

// /// Identify a specific drive on a bus, format: (bus, drive, model, serial. size, unit, sectors) 
// pub fn indentify_drive(bus : u8, drive : u8) -> Option<(u8, u8, String, String, u32, String, u32)> {
//     let mut buses = BUSES.lock();
//     if let Some(buf) = buses[bus as usize].identify_drive(drive) {
//         let mut serial = String::new();
//         for i in 10..20 {
//             for &b in &buf[i].to_be_bytes() {
//                 serial.push(b as char);
//             }
//         }
//         serial = serial.trim().into();
//         let mut model = String::new();
//         for i in 27..47 {
//             for &b in &buf[i].to_be_bytes() {
//                 model.push(b as char);
//             }
//         }
//         model = model.trim().into();
//         let sectors = (buf[61] as u32) << 16 | (buf[60] as u32);
//         let (size, unit) = disk_size(sectors);
//         Some((bus, drive, model, serial, size, unit, sectors))
//     } else {
//         None
//     } 
// }

// pub fn read(bus: u8, drive: u8, block: BlockIndex, buf: &mut [u8]) {
//     let mut buses = BUSES.lock();
//     //log!("Reading Block 0x{:08X}\n", block);
//     buses[bus as usize].read(drive, block, buf);
// }

// pub fn write(bus: u8, drive: u8, block: BlockIndex, buf : &[u8]) {
//     let mut buses = BUSES.lock();
//     //log!("Writing Block 0x{:08X}\n", block);
//     buses[bus as usize].write
//     (drive, block, buf);
// }



// pub fn drive_is_present(bus : usize) -> bool {
//     unsafe {BUSES.lock()[bus].status_register.read() != 0xFF}
// }



// pub fn init() -> Result<(), ()> {
//     {
//         let mut buses = BUSES.lock();
//         buses.push(Bus::new(0, 0x1F0, 0x3F6, 14));
//         buses.push(Bus::new(1, 0x170, 0x376, 15));
//     }
//     Ok(())
// }

//! COMES FROM https://wiki.osdev.org/PCI_IDE_Controller
// Status
// The Command/Status Port returns a bit mask referring to the status of a channel when read.

use core::ffi::{c_ushort, c_uchar, c_uint};

const ATA_SR_BSY: c_ushort = 0x80;    // Busy
const ATA_SR_DRDY: c_ushort = 0x40;    // Drive ready
const ATA_SR_DF: c_ushort = 0x20;    // Drive write fault
const ATA_SR_DSC: c_ushort = 0x10;    // Drive seek complete
const ATA_SR_DRQ: c_ushort = 0x08;    // Data request ready
const ATA_SR_CORR: c_ushort = 0x04;    // Corrected data
const ATA_SR_IDX: c_ushort = 0x02;    // Index
const ATA_SR_ERR: c_ushort = 0x01;    // Error
// Errors
// The Features/Error Port, which returns the most recent error upon read, has these possible bit masks

const ATA_ER_BBK: c_ushort = 0x80;    // Bad block
const ATA_ER_UNC: c_ushort = 0x40;    // Uncorrectable data
const ATA_ER_MC: c_ushort = 0x20;    // Media changed
const ATA_ER_IDNF: c_ushort = 0x10;    // ID mark not found
const ATA_ER_MCR: c_ushort = 0x08;    // Media change request
const ATA_ER_ABRT: c_ushort = 0x04;    // Command aborted
const ATA_ER_TK0NF: c_ushort = 0x02;    // Track 0 not found
const ATA_ER_AMNF: c_ushort = 0x01;    // No address mark
// Commands
// When you write to the Command/Status port, you are executing one of the commands below.

const ATA_CMD_READ_PIO: c_ushort = 0x20;
const ATA_CMD_READ_PIO_EXT: c_ushort = 0x24;
const ATA_CMD_READ_DMA: c_ushort = 0xC8;
const ATA_CMD_READ_DMA_EXT: c_ushort = 0x25;
const ATA_CMD_WRITE_PIO: c_ushort = 0x30;
const ATA_CMD_WRITE_PIO_EXT: c_ushort = 0x34;
const ATA_CMD_WRITE_DMA: c_ushort = 0xCA;
const ATA_CMD_WRITE_DMA_EXT: c_ushort = 0x35;
const ATA_CMD_CACHE_FLUSH: c_ushort = 0xE7;
const ATA_CMD_CACHE_FLUSH_EXT: c_ushort = 0xEA;
const ATA_CMD_PACKET: c_ushort = 0xA0;
const ATA_CMD_IDENTIFY_PACKET: c_ushort = 0xA1;
const ATA_CMD_IDENTIFY: c_ushort = 0xEC;
// The commands below are for ATAPI devices, which will be understood soon.

const      ATAPI_CMD_READ: c_ushort = 0xA8;
const      ATAPI_CMD_EJECT: c_ushort = 0x1B;
// ATA_CMD_IDENTIFY_PACKET and ATA_CMD_IDENTIFY return a buffer of 512 bytes called the identification space; the following definitions are used to read information from the identification space.

const ATA_IDENT_DEVICETYPE: c_ushort = 0;
const ATA_IDENT_CYLINDERS: c_ushort = 2;
const ATA_IDENT_HEADS: c_ushort = 6;
const ATA_IDENT_SECTORS: c_ushort = 12;
const ATA_IDENT_SERIAL: c_ushort = 20;
const ATA_IDENT_MODEL: c_ushort = 54;
const ATA_IDENT_CAPABILITIES: c_ushort = 98;
const ATA_IDENT_FIELDVALID: c_ushort = 106;
const ATA_IDENT_MAX_LBA: c_ushort = 120;
const ATA_IDENT_COMMANDSETS: c_ushort = 164;
const ATA_IDENT_MAX_LBA_EXT: c_ushort = 200;
// When you select a drive, you should specify the interface type and whether it is the master or slave:

const IDE_ATA: c_ushort = 0x00;
const IDE_ATAPI: c_ushort = 0x01;
 
const ATA_MASTER: c_ushort = 0x00;
const ATA_SLAVE: c_ushort = 0x01;
// Task File is a range of 8 ports which are offsets from BAR0 (primary channel) and/or BAR2 (secondary channel). To exemplify:

// BAR0 + 0 is first port.
// BAR0 + 1 is second port.
// BAR0 + 2 is the third
const ATA_REG_DATA: c_ushort = 0x00;
const ATA_REG_ERROR: c_ushort = 0x01;
const ATA_REG_FEATURES: c_ushort = 0x01;
const ATA_REG_SECCOUNT0: c_ushort = 0x02;
const ATA_REG_LBA0: c_ushort = 0x03;
const ATA_REG_LBA1: c_ushort = 0x04;
const ATA_REG_LBA2: c_ushort = 0x05;
const ATA_REG_HDDEVSEL: c_ushort = 0x06;
const ATA_REG_COMMAND: c_ushort = 0x07;
const ATA_REG_STATUS: c_ushort = 0x07;
const ATA_REG_SECCOUNT1: c_ushort = 0x08;
const ATA_REG_LBA3: c_ushort = 0x09;
const ATA_REG_LBA4: c_ushort = 0x0A;
const ATA_REG_LBA5: c_ushort = 0x0B;
const ATA_REG_CONTROL: c_ushort = 0x0C;
const ATA_REG_ALTSTATUS: c_ushort = 0x0C;
const ATA_REG_DEVADDRESS: c_ushort = 0x0D;
//; The ALTSTATUS/CONTROL port returns the alternate status when read and controls a channel when written to.

// For the primary channel, ALTSTATUS/CONTROL port is BAR1 + 2.
// For the secondary channel, ALTSTATUS/CONTROL port is BAR3 + 2.
// We can now say that each channel has 13 registers. For the primary channel, we use these values:

// Data Register: BAR0 + 0; // Read-Write
// Error Register: BAR0 + 1; // Read Only
// Features Register: BAR0 + 1; // Write Only
// SECCOUNT0: BAR0 + 2; // Read-Write
// LBA0: BAR0 + 3; // Read-Write
// LBA1: BAR0 + 4; // Read-Write
// LBA2: BAR0 + 5; // Read-Write
// HDDEVSEL: BAR0 + 6; // Read-Write, used to select a drive in the channel.
// Command Register: BAR0 + 7; // Write Only.
// Status Register: BAR0 + 7; // Read Only.
// Alternate Status Register: BAR1 + 2; // Read Only.
// Control Register: BAR1 + 2; // Write Only.
// DEVADDRESS: BAR1 + 3; // I don't know what is the benefit from this register.
// The map above is the same with the secondary channel, but it uses BAR2 and BAR3 instead of BAR0 and BAR1.

// Channels:
const ATA_PRIMARY: c_ushort = 0x00;
const ATA_SECONDARY: c_ushort = 0x01;
 
// Directions:
const ATA_READ: c_ushort = 0x00;
const ATA_WRITE: c_ushort = 0x01;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct IDEChannelRegisters {
    pub base: c_ushort,
    pub ctrl: c_ushort,
    pub bmide: c_ushort,
    pub n_ien: c_uchar,
}
pub static CHANNELS: [IDEChannelRegisters; 2] = [IDEChannelRegisters {
    base: 0,
    ctrl: 0,
    bmide: 0,
    n_ien: 0,
}; 2];


const IDE_BUFFER: [c_uchar; 2048] = [b'\0'; 2048];
static IDE_IRQ_INVOKED: c_uchar = b'\0';
static ATAPI_PACKET: [c_uchar; 12] = [0xA8 as c_uchar, b'\0',b'\0',b'\0',b'\0',b'\0',b'\0',b'\0',b'\0',b'\0',b'\0',b'\0'];


#[derive(Copy, Clone)]
#[repr(C)]
pub struct IdeDevice {
    pub reserved: c_uchar,
    pub channel: c_uchar,
    pub drive: c_uchar,
    pub _type: c_ushort,
    pub signature: c_ushort,
    pub capabilities: c_ushort,
    pub command_sets: c_uint,
    pub size: c_uint,
    pub model: [c_uchar; 41],
}
#[no_mangle]
pub static mut IDE_DEVICES: [IdeDevice; 4] = [IdeDevice {
    reserved: 0,
    channel: 0,
    drive: 0,
    _type: 0,
    signature: 0,
    capabilities: 0,
    command_sets: 0,
    size: 0,
    model: [0; 41],
}; 4];



fn ide_read(channel:usize, reg:u16) -> c_uchar {
    let result;
    if (reg > 0x07 && reg < 0x0C) {
       ide_write(channel, ATA_REG_CONTROL, 0x80 | CHANNELS[channel].n_ien);}
    if (reg < 0x08) {
       result = inb(CHANNELS[channel].base + reg);}
    else if (reg < 0x0C) {
       result = inb(CHANNELS[channel].base  + reg - 0x06);}
    else if (reg < 0x0E) {
       result = inb(CHANNELS[channel].ctrl  + reg - 0x0A);}
    else if (reg < 0x16) {
       result = inb(CHANNELS[channel].bmide + reg - 0x0E);}
    if (reg > 0x07 && reg < 0x0C) {
       ide_write(channel, ATA_REG_CONTROL, CHANNELS[channel].n_ien);}
    return result;
 }

 
fn ide_write(channel:usize, reg:u16, data:c_uchar) {//0x0C
    if (reg > 0x07 && reg < 0x0C) {
       ide_write(channel, ATA_REG_CONTROL, 0x80 | CHANNELS[channel].n_ien);}
    if (reg < 0x08) {
       outb(CHANNELS[channel].base  + reg - 0x00, data);}
    else if (reg < 0x0C) {
       outb(CHANNELS[channel].base  + reg - 0x06, data);}
    else if (reg < 0x0E) {
       outb(CHANNELS[channel].ctrl  + reg - 0x0A, data);}
    else if (reg < 0x16) {
       outb(CHANNELS[channel].bmide + reg - 0x0E, data);}
    if (reg > 0x07 && reg < 0x0C) {
       ide_write(channel, ATA_REG_CONTROL, CHANNELS[channel].n_ien);}
}
