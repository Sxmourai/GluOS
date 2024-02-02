use core::{mem::forget, ptr::{addr_of, slice_from_raw_parts}};

use alloc::vec::Vec;
use x86_64::{
    instructions::port::{Port, PortGeneric},
    structures::port::{PortRead, PortWrite},
};

use crate::{bit_manipulation::{read_at, write_at}, dbg, mem_handler, pci::PciDevice};

///! https://wiki.osdev.org/Intel_Ethernet_i217

const REG_CTRL: u16 = 0x0000;
const REG_STATUS: u16 = 0x0008;
const REG_EEPROM: u16 = 0x0014;
const REG_CTRL_EXT: u16 = 0x0018;
const REG_IMASK: u16 = 0x00D0;
const REG_RCTRL: u16 = 0x0100;
const REG_RXDESCLO: u16 = 0x2800;
const REG_RXDESCHI: u16 = 0x2804;
const REG_RXDESCLEN: u16 = 0x2808;
const REG_RXDESCHEAD: u16 = 0x2810;
const REG_RXDESCTAIL: u16 = 0x2818;

const REG_TCTRL: u16 = 0x0400;
const REG_TXDESCLO: u16 = 0x3800;
const REG_TXDESCHI: u16 = 0x3804;
const REG_TXDESCLEN: u16 = 0x3808;
const REG_TXDESCHEAD: u16 = 0x3810;
const REG_TXDESCTAIL: u16 = 0x3818;

const REG_RDTR: u16 = 0x2820; // RX Delay Timer Register
const REG_RXDCTL: u16 = 0x2828; // RX Descriptor Control
const REG_RADV: u16 = 0x282C; // RX Int. Absolute Delay Timer
const REG_RSRPD: u16 = 0x2C00; // RX Small Packet Detect Interrupt

const REG_TIPG: u16 = 0x0410; // Transmit Inter Packet Gap
const ECTRL_SLU: u16 = 0x40; //set link up

const RCTL_EN: u16 = (1 << 1); // Receiver Enable
const RCTL_SBP: u16 = (1 << 2); // Store Bad Packets
const RCTL_UPE: u16 = (1 << 3); // Unicast Promiscuous Enabled
const RCTL_MPE: u16 = (1 << 4); // Multicast Promiscuous Enabled
const RCTL_LPE: u16 = (1 << 5); // Long Packet Reception Enable
const RCTL_LBM_NONE: u16 = (0 << 6); // No Loopback
const RCTL_LBM_PHY: u16 = (3 << 6); // PHY or external SerDesc loopback
const RTCL_RDMTS_HALF: u16 = (0 << 8); // Free Buffer Threshold is 1/2 of RDLEN
const RTCL_RDMTS_QUARTER: u16 = (1 << 8); // Free Buffer Threshold is 1/4 of RDLEN
const RTCL_RDMTS_EIGHTH: u16 = (2 << 8); // Free Buffer Threshold is 1/8 of RDLEN
const RCTL_MO_36: u16 = (0 << 12); // Multicast Offset - bits 47:36
const RCTL_MO_35: u16 = (1 << 12); // Multicast Offset - bits 46:35
const RCTL_MO_34: u16 = (2 << 12); // Multicast Offset - bits 45:34
const RCTL_MO_32: u16 = (3 << 12); // Multicast Offset - bits 43:32
const RCTL_BAM: u16 = (1 << 15); // Broadcast Accept Mode
const RCTL_VFE: u32 = (1 << 18); // VLAN Filter Enable
const RCTL_CFIEN: u32 = (1 << 19); // Canonical Form Indicator Enable
const RCTL_CFI: u32 = (1 << 20); // Canonical Form Indicator Bit Value
const RCTL_DPF: u32 = (1 << 22); // Discard Pause Frames
const RCTL_PMCF: u32 = (1 << 23); // Pass MAC Control Frames
const RCTL_SECRC: u32 = (1 << 26); // Strip Ethernet CRC

// Buffer Sizes
const RCTL_BSIZE_256: u32 = (3 << 16);
const RCTL_BSIZE_512: u32 = (2 << 16);
const RCTL_BSIZE_1024: u32 = (1 << 16);
const RCTL_BSIZE_2048: u32 = (0 << 16);
const RCTL_BSIZE_4096: u32 = ((3 << 16) | (1 << 25));
const RCTL_BSIZE_8192: u32 = ((2 << 16) | (1 << 25));
const RCTL_BSIZE_16384: u32 = ((1 << 16) | (1 << 25));

// Transmit Command
const CMD_EOP: u16 = (1 << 0); // End of Packet
const CMD_IFCS: u16 = (1 << 1); // Insert FCS
const CMD_IC: u16 = (1 << 2); // Insert Checksum
const CMD_RS: u16 = (1 << 3); // Report Status
const CMD_RPS: u16 = (1 << 4); // Report Packet Sent
const CMD_VLE: u16 = (1 << 6); // VLAN Packet Enable
const CMD_IDE: u16 = (1 << 7); // Interrupt Delay Enable

// TCTL Register
const TCTL_EN: u16 = (1 << 1); // Transmit Enable
const TCTL_PSP: u16 = (1 << 3); // Pad Short Packets
const TCTL_CT_SHIFT: u16 = 4; // Collision Threshold
const TCTL_COLD_SHIFT: u16 = 12; // Collision Distance
const TCTL_SWXOFF: u32 = (1 << 22); // Software XOFF Transmission
const TCTL_RTLC: u32 = (1 << 24); // Re-transmit on Late Collision

const TSTA_DD: u8 = (1 << 0); // Descriptor Done
const TSTA_EC: u8 = (1 << 1); // Excess Collisions
const TSTA_LC: u8 = (1 << 2); // Late Collision
const LSTA_TU: u8 = (1 << 3); // Transmit Underrun

const E1000_NUM_RX_DESC: u16 = 32;
const E1000_NUM_TX_DESC: u16 = 8;

#[repr(packed)]
#[derive(Default)]
struct E1000RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(packed)]
#[derive(Default)]
struct E1000TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}
#[derive(Default)]
pub struct E1000NetworkDriver {
    /// Type of BAR0
    bar_type: u8,
    /// IO Base Address
    io_base: u16,
    /// MMIO Base Address
    mem_base: u64,
    /// A flag indicating if eeprom exists
    eerprom_exists: bool,
    /// A buffer for storing the mack address  
    mac: [u8; 6],
    /// Receive Descriptor Buffers
    rx_descs: [E1000RxDesc; E1000_NUM_RX_DESC as usize],
    /// Transmit Descriptor Buffers
    tx_descs: [E1000TxDesc; E1000_NUM_TX_DESC as usize],
    /// Current Receive Descriptor Buffer
    rx_cur: u16,
    /// Current Transmit Descriptor Buffer
    tx_cur: u16,
}
impl E1000NetworkDriver {
    pub fn new(pci_device: &PciDevice) -> Self {
        //TODO Enable bus mastering
        // Off course you will need here to map the memory address into you page tables and use corresponding virtual addresses
        // log::debug!("{:#b} {:#b}", pci_device.raw.bars[0], pci_device.raw.bars[0] & (1<<30|1<<31));
        Self {
            // Get BAR0 type, io_base address and MMIO base address
            bar_type: if (pci_device.raw.bars[0] & (1<<30|1<<31))!=0 {1} else {0} as u8,
            io_base: (pci_device.raw.bars[1] & !1).try_into().unwrap(),
            mem_base: (pci_device.raw.bars[2]& !3).try_into().unwrap(),
            eerprom_exists: false,
            ..core::default::Default::default()
        }
    }
    //TODO return result
    pub fn start(&mut self) -> Result<(), E1000NetworkDriverInitError> {
        self.eerprom_exists = self.detect_ee_prom();
        dbg!(self.eerprom_exists);
        self.eerprom_exists = true;
        self.read_mac_addr().or(Err(E1000NetworkDriverInitError::CantReadMac))?;
        dbg!(self.mac);
        self.start_link();
        dbg!(self.mac);
 
        for i in 0..0x80 {
            self.write_command(0x5200 + i*4, 0);
        }
        // if interruptManager->registerInterrupt(IRQ0+pciConfigHeader->getIntLine(),this))
        // {
            self.enable_interrupts();
            self.rx_init();
            self.tx_init();
            dbg!("E1000 Started !");
            return Ok(());
        // }
    }
    pub fn fire(&self) {
        todo!()
    }
    pub fn send_packet(p_data: &[u8]) -> Result<(), PacketSendError> {
        todo!()
    }
    /// Send Commands and read results From NICs either using MMIO or IO Ports
    fn write_command(&self,p_addr: u16, p_value: u32) {
        if (self.bar_type == 0) {
            unsafe{
                write_at::<u32>(self.mem_base as usize + p_addr as usize, p_value)
            };
        } else {
            dbg!(self.bar_type);
            unsafe {
                PortWrite::write_to_port(self.io_base, p_addr as u32);
                PortWrite::write_to_port(self.io_base + 4, p_value);
            }
        }
    }
    fn read_command(&self, p_addr: u16) -> u32 {
        if (self.bar_type == 0) {
            unsafe { read_at::<u32>(self.mem_base as usize + p_addr as usize) }
        } else {
            unsafe {
                PortWrite::write_to_port(self.io_base, p_addr as u32);
                PortRead::read_from_port(self.io_base + 4)
            }
        }
    }

    /// Return true if EEProm exist, else it returns false and set the eerprom_existsdata member
    fn detect_ee_prom(&self) -> bool {
        let mut val = 0;
        self.write_command(REG_EEPROM, 0x1); 
        for i in 0..1000 {
            val = self.read_command(REG_EEPROM);
            if val & 0x10!=0 {
                return true;
            }
        }
        false
    }
    /// Read 4 bytes from a specific EEProm Address
    fn ee_prom_read(&self, addr: u8) -> u32 {
        let mut tmp = 0;
        if self.eerprom_exists {
            self.write_command(REG_EEPROM, (1) | ((addr as u32) << 8) );
        	loop {
                tmp = self.read_command(REG_EEPROM);
                log::debug!("{:#b}", tmp);
                if tmp & (1<<4)!=0 {break}
            }
        }
        else {
            self.write_command(REG_EEPROM, (1) | ((addr as u32) << 2) );
            loop {
                tmp = self.read_command(REG_EEPROM);
                if tmp & (1<<1)!=0 {break}
            }
        }
        (tmp >> 16) & 0xFFFF
    }
    /// Read MAC Address and returns true if success
    /// TODO Return better result
    fn read_mac_addr(&mut self) -> Result<(), E1000ReadMac> {
        if self.eerprom_exists {
            let mut temp;
            temp = self.ee_prom_read(0);
            self.mac[0] = (temp &0xff) as u8;
            self.mac[1] = (temp >> 8) as u8;
            temp = self.ee_prom_read(1);
            self.mac[2] = (temp &0xff) as u8;
            self.mac[3] = (temp >> 8) as u8;
            temp = self.ee_prom_read(2);
            self.mac[4] = (temp &0xff) as u8;
            self.mac[5] = (temp >> 8) as u8;
        }
        else {
            let raw_mem_base_mac = slice_from_raw_parts((self.mem_base+0x5400) as *const u32, 6);
            let mem_base_mac = unsafe {&*raw_mem_base_mac};
            let raw_mem_base_macu8 = slice_from_raw_parts((self.mem_base+0x5400) as *const u8, 6);
            let mem_base_macu8 = unsafe {&*raw_mem_base_macu8};
            if ( mem_base_mac[0] != 0 ) {
                for i in 0..6 {
                    self.mac[i] = mem_base_macu8[i];
                }
            } else {
                return Err(E1000ReadMac::NoMemoryBase);
            }
        }
        Ok(())
    }
    /// Start up the network
    fn start_link(&mut self) {
        todo!()
    }
    /// Initialize receive descriptors and buffers
    fn rx_init(&mut self) {
        for i in 0..E1000_NUM_RX_DESC {
            let mut desc_vec = Vec::with_capacity(8192+16);
            let ptr: &'static mut [u8] = desc_vec.leak();
            let desc_ptr = addr_of!(ptr);
            self.rx_descs[i as usize].addr = desc_ptr as u64;
            self.rx_descs[i as usize].status = 0;
        }
        let ptr = addr_of!(self.rx_descs);
        self.write_command(REG_TXDESCLO, ((ptr as u64) >> 32) as u32);
        self.write_command(REG_TXDESCHI, ((ptr as u64) & 0xFFFFFFFF) as u32);
    
        self.write_command(REG_RXDESCLO, ptr as u32);
        self.write_command(REG_RXDESCHI, 0);
    
        self.write_command(REG_RXDESCLEN, E1000_NUM_RX_DESC as u32 * 16);
    
        self.write_command(REG_RXDESCHEAD, 0);
        self.write_command(REG_RXDESCTAIL, E1000_NUM_RX_DESC as u32-1);
        self.rx_cur = 0;
        self.write_command(REG_RCTRL, (RCTL_EN| RCTL_SBP| RCTL_UPE | RCTL_MPE | RCTL_LBM_NONE | RTCL_RDMTS_HALF | RCTL_BAM) as u32 | RCTL_SECRC  | RCTL_BSIZE_8192);
    }
    /// Initialize transmit descriptors and buffers
    fn tx_init(&mut self) {
        for i in 0..E1000_NUM_TX_DESC {
            self.tx_descs[i as usize].addr = 0;
            self.tx_descs[i as usize].cmd = 0;
            self.tx_descs[i as usize].status = TSTA_DD;
        }
        let ptr = addr_of!(self.tx_descs);
        self.write_command(REG_TXDESCHI, ((ptr as u64) >> 32) as u32);
        self.write_command(REG_TXDESCLO, ((ptr as u64) & 0xFFFFFFFF) as u32);
        //now setup total length of descriptors
        self.write_command(REG_TXDESCLEN, E1000_NUM_TX_DESC as u32 * 16);
        //setup numbers
        self.write_command( REG_TXDESCHEAD, 0);
        self.write_command( REG_TXDESCTAIL, 0);
        self.tx_cur = 0;
        self.write_command(REG_TCTRL,  (TCTL_EN | TCTL_PSP) as u32 | (15 << TCTL_CT_SHIFT) | (64 << TCTL_COLD_SHIFT) | TCTL_RTLC);
        // This line of code overrides the one before it but I left both to highlight that the previous one works with e1000 cards, but for the e1000e cards 
        // you should set the TCTRL register as follows. For detailed description of each bit, please refer to the Intel Manual.
        // In the case of I217 and 82577LM packets will not be sent if the TCTRL is not configured using the following bits.
        self.write_command(REG_TCTRL,  0b0110000000000111111000011111010);
        self.write_command(REG_TIPG,  0x0060200A);
    }

    fn enable_interrupts(&mut self) {
        self.write_command(REG_IMASK ,0x1F6DC);
        self.write_command(REG_IMASK ,0xff & !4);
        self.read_command(0xc0);
    }
    /// Handle a packet reception
    fn handle_receive(&mut self) {
        let mut old_cur = 0;
        let mut got_packet = false;
    
        while (self.rx_descs[self.rx_cur as usize].status & 0x1!=0) {
            got_packet = true;
            let buf = self.rx_descs[self.rx_cur as usize].addr;
            let len = self.rx_descs[self.rx_cur as usize].length;

            // Here you should inject the received packet into your network stack


            self.rx_descs[self.rx_cur as usize].status = 0;
            old_cur = self.rx_cur;
            self.rx_cur = (self.rx_cur + 1) % E1000_NUM_RX_DESC;
            self.write_command(REG_RXDESCTAIL, old_cur as u32);
        }    
    }
}
pub enum E1000ReadMac {
    NoMemoryBase
}
pub enum E1000NetworkDriverInitError {
    CantReadMac
}


pub enum PacketSendError {}
