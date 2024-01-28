use x86_64::{
    instructions::port::{Port, PortGeneric},
    structures::port::{PortRead, PortWrite},
};

use crate::bit_manipulation::{read_at, write_at};

///! https://wiki.osdev.org/Intel_Ethernet_i217

const REG_CTRL: u32 = 0x0000;
const REG_STATUS: u32 = 0x0008;
const REG_EEPROM: u32 = 0x0014;
const REG_CTRL_EXT: u32 = 0x0018;
const REG_IMASK: u32 = 0x00D0;
const REG_RCTRL: u32 = 0x0100;
const REG_RXDESCLO: u32 = 0x2800;
const REG_RXDESCHI: u32 = 0x2804;
const REG_RXDESCLEN: u32 = 0x2808;
const REG_RXDESCHEAD: u32 = 0x2810;
const REG_RXDESCTAIL: u32 = 0x2818;

const REG_TCTRL: u32 = 0x0400;
const REG_TXDESCLO: u32 = 0x3800;
const REG_TXDESCHI: u32 = 0x3804;
const REG_TXDESCLEN: u32 = 0x3808;
const REG_TXDESCHEAD: u32 = 0x3810;
const REG_TXDESCTAIL: u32 = 0x3818;

const REG_RDTR: u32 = 0x2820; // RX Delay Timer Register
const REG_RXDCTL: u32 = 0x2828; // RX Descriptor Control
const REG_RADV: u32 = 0x282C; // RX Int. Absolute Delay Timer
const REG_RSRPD: u32 = 0x2C00; // RX Small Packet Detect Interrupt

const REG_TIPG: u32 = 0x0410; // Transmit Inter Packet Gap
const ECTRL_SLU: u32 = 0x40; //set link up

const RCTL_EN: u32 = (1 << 1); // Receiver Enable
const RCTL_SBP: u32 = (1 << 2); // Store Bad Packets
const RCTL_UPE: u32 = (1 << 3); // Unicast Promiscuous Enabled
const RCTL_MPE: u32 = (1 << 4); // Multicast Promiscuous Enabled
const RCTL_LPE: u32 = (1 << 5); // Long Packet Reception Enable
const RCTL_LBM_NONE: u32 = (0 << 6); // No Loopback
const RCTL_LBM_PHY: u32 = (3 << 6); // PHY or external SerDesc loopback
const RTCL_RDMTS_HALF: u32 = (0 << 8); // Free Buffer Threshold is 1/2 of RDLEN
const RTCL_RDMTS_QUARTER: u32 = (1 << 8); // Free Buffer Threshold is 1/4 of RDLEN
const RTCL_RDMTS_EIGHTH: u32 = (2 << 8); // Free Buffer Threshold is 1/8 of RDLEN
const RCTL_MO_36: u32 = (0 << 12); // Multicast Offset - bits 47:36
const RCTL_MO_35: u32 = (1 << 12); // Multicast Offset - bits 46:35
const RCTL_MO_34: u32 = (2 << 12); // Multicast Offset - bits 45:34
const RCTL_MO_32: u32 = (3 << 12); // Multicast Offset - bits 43:32
const RCTL_BAM: u32 = (1 << 15); // Broadcast Accept Mode
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

const CMD_EOP: u32 = (1 << 0); // End of Packet
const CMD_IFCS: u32 = (1 << 1); // Insert FCS
const CMD_IC: u32 = (1 << 2); // Insert Checksum
const CMD_RS: u32 = (1 << 3); // Report Status
const CMD_RPS: u32 = (1 << 4); // Report Packet Sent
const CMD_VLE: u32 = (1 << 6); // VLAN Packet Enable
const CMD_IDE: u32 = (1 << 7); // Interrupt Delay Enable

// TCTL Register

const TCTL_EN: u32 = (1 << 1); // Transmit Enable
const TCTL_PSP: u32 = (1 << 3); // Pad Short Packets
const TCTL_CT_SHIFT: u32 = 4; // Collision Threshold
const TCTL_COLD_SHIFT: u32 = 12; // Collision Distance
const TCTL_SWXOFF: u32 = (1 << 22); // Software XOFF Transmission
const TCTL_RTLC: u32 = (1 << 24); // Re-transmit on Late Collision

const TSTA_DD: u32 = (1 << 0); // Descriptor Done
const TSTA_EC: u32 = (1 << 1); // Excess Collisions
const TSTA_LC: u32 = (1 << 2); // Late Collision
const LSTA_TU: u32 = (1 << 3); // Transmit Underrun

const E1000_NUM_RX_DESC: u32 = 32;
const E1000_NUM_TX_DESC: u32 = 8;

#[repr(packed)]
struct E1000RxDesc {
    addr: u64,
    length: u16,
    checksum: u16,
    status: u8,
    errors: u8,
    special: u16,
}

#[repr(packed)]
struct E1000TxDesc {
    addr: u64,
    length: u16,
    cso: u8,
    cmd: u8,
    status: u8,
    css: u8,
    special: u16,
}

struct E1000NetworkDriver {
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
    tx_descs: [E1000RxDesc; E1000_NUM_TX_DESC as usize],
    /// Current Receive Descriptor Buffer
    rx_cur: u16,
    /// Current Transmit Descriptor Buffer
    tx_cur: u16,
}
trait NetworkDriver {
    fn addr(&self) -> usize;
    fn mem_base(&self) -> usize;
    fn bar_type(&self) -> usize;
    fn io_base(&self) -> u16;
    /// Send Commands and read results From NICs either using MMIO or IO Ports
    fn write_command(&self, p_value: u32) {
        if (self.bar_type() == 0) {
            write_at::<u32>(self.mem_base() + self.addr(), p_value);
        } else {
            unsafe {
                PortWrite::write_to_port(self.io_base(), self.addr() as u32);
                PortWrite::write_to_port(self.io_base() + 4, p_value);
            }
        }
    }
    fn read_command(&self) -> u32 {
        if (self.bar_type() == 0) {
            unsafe { read_at::<u32>(self.mem_base() + self.addr()) }
        } else {
            unsafe {
                PortWrite::write_to_port(self.io_base(), self.addr() as u32);
                PortRead::read_from_port(self.io_base() + 4)
            }
        }
    }

    /// Return true if EEProm exist, else it returns false and set the eerprom_existsdata member
    fn detect_ee_prom(&self) -> bool;
    /// Read 4 bytes from a specific EEProm Address
    fn ee_prom_read(&self) -> u32;
    /// Read MAC Address
    fn read_mac_addr(&self) -> bool;
    /// Start up the network
    fn start_link(&mut self);
    /// Initialize receive descriptors and buffers
    fn rx_init(&mut self);
    /// Initialize transmit descriptors and buffers
    fn tx_init(&mut self);

    fn enable_interrupts(&mut self);
    /// Handle a packet reception.
    fn handle_receive(&mut self);

    // ---- Public interface ----
    fn start(&mut self);
    //TODO fn fire()
    fn get_mac_addr(&self) -> u8;

    fn send_packet(p_data: &[u8]) -> Result<(), PacketSendError>;
}

pub enum PacketSendError {}
