// FROM https://wiki.osdev.org/PCI_IDE_Controller

use core::ffi::{c_uchar, c_ushort, c_uint};

use volatile::Volatile;

// Status
// The Command/Status Port returns a bit mask referring to the status of a channel when read.
const ATA_SR_BSY:              u32 =  0x80;    // Busy
const ATA_SR_DRDY:             u32 =  0x40;    // Drive ready
const ATA_SR_DF:               u32 =  0x20;    // Drive write fault
const ATA_SR_DSC:              u32 =  0x10;    // Drive seek complete
const ATA_SR_DRQ:              u32 =  0x08;    // Data request ready
const ATA_SR_CORR:             u32 =  0x04;    // Corrected data
const ATA_SR_IDX:              u32 =  0x02;    // Index
const ATA_SR_ERR:              u32 =  0x01;    // Error

// Errors
// The Features/Error Port, which returns the most recent error upon read, has these possible bit masks
const ATA_ER_BBK:              u32 = 0x80;    // Bad block
const ATA_ER_UNC:              u32 = 0x40;    // Uncorrectable data
const ATA_ER_MC:               u32 = 0x20;    // Media changed
const ATA_ER_IDNF:             u32 = 0x10;    // ID mark not found
const ATA_ER_MCR:              u32 = 0x08;    // Media change request
const ATA_ER_ABRT:             u32 = 0x04;    // Command aborted
const ATA_ER_TK0NF:            u32 = 0x02;    // Track 0 not found
const ATA_ER_AMNF:             u32 = 0x01;    // No address mark

// Commands
// When you write to the Command/Status port, you are executing one of the commands below.
const ATA_CMD_READ_PIO:        u32 = 0x20;
const ATA_CMD_READ_PIO_EXT:    u32 = 0x24;
const ATA_CMD_READ_DMA:        u32 = 0xC8;
const ATA_CMD_READ_DMA_EXT:    u32 = 0x25;
const ATA_CMD_WRITE_PIO:       u32 = 0x30;
const ATA_CMD_WRITE_PIO_EXT:   u32 = 0x34;
const ATA_CMD_WRITE_DMA:       u32 = 0xCA;
const ATA_CMD_WRITE_DMA_EXT:   u32 = 0x35;
const ATA_CMD_CACHE_FLUSH:     u32 = 0xE7;
const ATA_CMD_CACHE_FLUSH_EXT: u32 = 0xEA;
const ATA_CMD_PACKET:          u32 = 0xA0;
const ATA_CMD_IDENTIFY_PACKET: u32 = 0xA1;
const ATA_CMD_IDENTIFY:        u32 = 0xEC;

// The commands below are for ATAPI devices, which will be understood soon.
const ATAPI_CMD_READ:               u32 = 0xA8;
const ATAPI_CMD_EJECT:              u32 = 0x1B;

// ATA_CMD_IDENTIFY_PACKET and ATA_CMD_IDENTIFY return a buffer of 512 bytes called the identification space; the following definitions are used to read information from the identification space.
const ATA_IDENT_DEVICETYPE:         u32 = 0;
const ATA_IDENT_CYLINDERS:          u32 = 2;
const ATA_IDENT_HEADS:              u32 = 6;
const ATA_IDENT_SECTORS:            u32 = 12;
const ATA_IDENT_SERIAL:             u32 = 20;
const ATA_IDENT_MODEL:              u32 = 54;
const ATA_IDENT_CAPABILITIES:       u32 = 98;
const ATA_IDENT_FIELDVALID:         u32 = 106;
const ATA_IDENT_MAX_LBA:            u32 = 120;
const ATA_IDENT_COMMANDSETS:        u32 = 164;
const ATA_IDENT_MAX_LBA_EXT:        u32 = 200;

// When you select a drive, you should specify the interface type and whether it is the master or slave:
const IDE_ATA:                      u32 = 0x00;
const IDE_ATAPI:                    u32 = 0x01;
 
const ATA_MASTER:                   u32 = 0x00;
const ATA_SLAVE:                    u32 = 0x01;

// Task File is a range of 8 ports which are offsets from BAR0 (primary channel) and/or BAR2 (secondary channel). To exemplify:
// - BAR0 + 0 is first port.
// - BAR0 + 1 is second port.
// - BAR0 + 2 is the third
const ATA_REG_DATA:                 u32 = 0x00;
const ATA_REG_ERROR:                u32 = 0x01;
const ATA_REG_FEATURES:             u32 = 0x01;
const ATA_REG_SECCOUNT0:            u32 = 0x02;
const ATA_REG_LBA0:                 u32 = 0x03;
const ATA_REG_LBA1:                 u32 = 0x04;
const ATA_REG_LBA2:                 u32 = 0x05;
const ATA_REG_HDDEVSEL:             u32 = 0x06;
const ATA_REG_COMMAND:              u32 = 0x07;
const ATA_REG_STATUS:               u32 = 0x07;
const ATA_REG_SECCOUNT1:            u32 = 0x08;
const ATA_REG_LBA3:                 u32 = 0x09;
const ATA_REG_LBA4:                 u32 = 0x0A;
const ATA_REG_LBA5:                 u32 = 0x0B;
const ATA_REG_CONTROL:              u32 = 0x0C;
const ATA_REG_ALTSTATUS:            u32 = 0x0C;
const ATA_REG_DEVADDRESS:           u32 = 0x0D;


// The ALTSTATUS/CONTROL port returns the alternate status when read and controls a channel when written to.
// 
// For the primary channel, ALTSTATUS/CONTROL port is BAR1 + 2.
// For the secondary channel, ALTSTATUS/CONTROL port is BAR3 + 2.
// We can now say that each channel has 13 registers. For the primary channel, we use these values:
/*
Data Register: BAR0 + 0;              // Read-Write
Error Register: BAR0 + 1;             // Read Only
Features Register: BAR0 + 1;          // Write Only
SECCOUNT0: BAR0 + 2;                  // Read-Write
LBA0: BAR0 + 3;                       // Read-Write
LBA1: BAR0 + 4;                       // Read-Write
LBA2: BAR0 + 5;                       // Read-Write
HDDEVSEL: BAR0 + 6;                   // Read-Write, used to select a drive in the channel.
Command Register: BAR0 + 7;           // Write Only.
Status Register: BAR0 + 7;            // Read Only.
Alternate Status Register: BAR1 + 2;  // Read Only.
Control Register: BAR1 + 2;           // Write Only.
DEVADDRESS: BAR1 + 3;                 // I don't know what is the benefit from this register.
The map above is the same with the secondary channel, but it uses BAR2 and BAR3 instead of BAR0 and BAR1.
*/
// Channels:
const ATA_PRIMARY:     u32 = 0x00;
const ATA_SECONDARY:   u32 = 0x01;
 
// Directions:
const ATA_READ:        u32 = 0x00;
const ATA_WRITE:       u32 = 0x01;

/*
We have defined everything needed by the driver, now lets move to an important part. We said that

BAR0 is the start of the I/O ports used by the primary channel.
BAR1 is the start of the I/O ports which control the primary channel.
BAR2 is the start of the I/O ports used by secondary channel.
BAR3 is the start of the I/O ports which control secondary channel.
BAR4 is the start of 8 I/O ports controls the primary channel's Bus Master IDE.
BAR4 + 8 is the Base of 8 I/O ports controls secondary channel's Bus Master IDE.
*/
#[derive(Copy, Clone)]
#[repr(C)]
pub struct IDEChannelRegisters {
    pub base: c_ushort,
    pub ctrl: c_ushort,
    pub bmide:c_ushort,
    pub n_ien: c_uchar,
}
#[no_mangle]
pub static mut channels: [IDEChannelRegisters; 2] = [IDEChannelRegisters {
    base: 0,
    ctrl: 0,
    bmide: 0,
    n_ien: 0,
}; 2];
// We also need a buffer to read the identification space into, we need a variable that indicates if an irq is invoked or not, and finally we need an array of 6 words [12 bytes] for ATAPI Drives:

const IDE_BUF: [char; 2048] = ['\0';2048];
static IDE_IRQ_INVOKED: char = '\0'; // Volatile
static ATAPI_PACKET: [char; 12] = [0xA8 as char, '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0'];

// We said the the IDE can contain up to 4 drives:
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

// When we read a register in a channel, like STATUS Register, it is easy to execute:
// ide_read(channel, ATA_REG_STATUS);
 
fn ide_read(channel: c_uchar, reg: c_uchar) -> c_uchar {
    let schan = channel as usize;
    let mut result: c_uchar;
    if (reg > 0x07 && reg < 0x0C) {
        ide_write(channel, ATA_REG_CONTROL as u8, 0x80 | unsafe { channels }[schan].n_ien);
    }
    if (reg < 0x08) {
        result = inb(unsafe { channels }[schan].base + reg as u16 - 0x00);
    }
    else if (reg < 0x0C) {
        result = inb(unsafe { channels }[schan].base  + reg as u16 - 0x06);
    }
    else if (reg < 0x0E) {
        result = inb(unsafe { channels }[schan].ctrl  + reg as u16 - 0x0A);
    }
    else if (reg < 0x16) {
        result = inb(unsafe { channels }[schan].bmide + reg as u16 - 0x0E);
    }
    if (reg > 0x07 && reg < 0x0C) {
        ide_write(channel, ATA_REG_CONTROL as u8, unsafe { channels }[schan].n_ien);
    }
    return result;
}

// We also need a function for writing to registers:
fn ide_write(channel: c_uchar, reg: c_uchar, data: c_uchar) {
   if (reg > 0x07 && reg < 0x0C){
      ide_write(channel, ATA_REG_CONTROL as u8, 0x80 | unsafe { channels }[channel as usize].n_ien);}
   if (reg < 0x08){
      outb(unsafe { channels }[channel as usize].base  + reg as u16 - 0x00, data);}
   else if (reg < 0x0C){
      outb(unsafe { channels }[channel as usize].base  + reg as u16 - 0x06, data);}
   else if (reg < 0x0E){
      outb(unsafe { channels }[channel as usize].ctrl  + reg as u16 - 0x0A, data);}
   else if (reg < 0x16){
      outb(unsafe { channels }[channel as usize].bmide + reg as u16 - 0x0E, data);}
   if (reg > 0x07 && reg < 0x0C){
      ide_write(channel, ATA_REG_CONTROL as u8, unsafe { channels }[channel as usize].n_ien);}
}