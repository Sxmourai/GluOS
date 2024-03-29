// COPY PASTE FROM https://www.theseus-os.com/Theseus/doc/src/pci/lib.rs.html

// Usefull links: all of classes: https://pci-ids.ucw.cz/read/PD/
// Intel devices : https://pci-ids.ucw.cz/read/PC/8086

pub mod port;

use alloc::string::String;
use alloc::vec::Vec;
use bit_field::BitField;
use core::fmt;
use core::ops::{Deref, DerefMut};
use hashbrown::HashMap;
use pci_ids::{Class, Classes, FromId, SubSystem, Subclass};
use spin::{Mutex, Once, RwLock};
use x86_64::instructions::port::Port;
use x86_64::{PhysAddr, VirtAddr};

use crate::{dbg, serial_println};

// The below constants define the PCI configuration space.
// More info here: <http://wiki.osdev.org/PCI#PCI_Device_Structure>
pub const PCI_VENDOR_ID: u8 = 0x0;
pub const PCI_DEVICE_ID: u8 = 0x2;
pub const PCI_COMMAND: u8 = 0x4;
pub const PCI_STATUS: u8 = 0x6;
pub const PCI_REVISION_ID: u8 = 0x8;
pub const PCI_PROG_IF: u8 = 0x9;
pub const PCI_SUBCLASS: u8 = 0xA;
pub const PCI_CLASS: u8 = 0xB;
pub const PCI_CACHE_LINE_SIZE: u8 = 0xC;
pub const PCI_LATENCY_TIMER: u8 = 0xD;
pub const PCI_HEADER_TYPE: u8 = 0xE;
pub const PCI_BIST: u8 = 0xF;
pub const PCI_BAR0: u8 = 0x10;
pub const PCI_BAR1: u8 = 0x14;
pub const PCI_BAR2: u8 = 0x18;
pub const PCI_BAR3: u8 = 0x1C;
pub const PCI_BAR4: u8 = 0x20;
pub const PCI_BAR5: u8 = 0x24;
pub const PCI_CARDBUS_CIS: u8 = 0x28;
pub const PCI_SUBSYSTEM_VENDOR_ID: u8 = 0x2C;
pub const PCI_SUBSYSTEM_ID: u8 = 0x2E;
pub const PCI_EXPANSION_ROM_BASE: u8 = 0x30;
pub const PCI_CAPABILITIES: u8 = 0x34;
// 0x35 through 0x3B are reserved
pub const PCI_INTERRUPT_LINE: u8 = 0x3C;
pub const PCI_INTERRUPT_PIN: u8 = 0x3D;
pub const PCI_MIN_GRANT: u8 = 0x3E;
pub const PCI_MAX_LATENCY: u8 = 0x3F;

pub type PciManager = HashMap<PciLocation, PciDevice>;
pub static mut MANAGER: Option<PciManager> = None;
#[macro_export]
macro_rules! pci_manager {
    () => {
        unsafe { &$crate::drivers::pci::MANAGER.as_ref().unwrap() }
    };
}

pub struct PciDevice {
    pub raw: &'static RawPciDevice,
    pub identified: &'static pci_ids::Device,
    pub class: &'static pci_ids::Class,
}
impl PciDevice {
    #[must_use] pub fn location(&self) -> PciLocation {
        self.raw.location
    }
    #[must_use] pub fn subclass(&self) -> u8 {
        self.raw.subclass
    }
    #[must_use] pub fn vendor_id(&self) -> u16 {
        self.raw.vendor_id
    }
    #[must_use] pub fn device_id(&self) -> u16 {
        self.raw.device_id
    }
    #[must_use] pub fn command(&self) -> u16 {
        self.raw.command
    }
    #[must_use] pub fn status(&self) -> u16 {
        self.raw.status
    }
    #[must_use] pub fn vendor(&self) -> &pci_ids::Vendor {
        self.identified.vendor()
    }
    #[must_use] pub fn name(&self) -> &'static str {
        self.identified.name()
    }
    pub fn subsystems(&self) -> impl Iterator<Item = &'static pci_ids::SubSystem> {
        self.identified.subsystems()
    }
    #[must_use] pub fn class(&self) -> &'static pci_ids::Class {
        self.class
    }
    #[must_use] pub fn display_classes(&self) -> String {
        let mut classes = alloc::format!("Class: {}", self.class().name());
        if let Some(subclass) = self
            .class()
            .subclasses()
            .find(|sub| sub.id() == self.raw.subclass)
        {
            classes.push_str(alloc::format!(" - Subclass: {}", subclass.name()).as_str());
        }
        classes
    }
}
impl core::fmt::Display for PciDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_fmt(format_args!(
            "{}.{}.{} ({:#x}) - {} {:?}",
            self.location().bus(),
            self.location().slot(),
            self.location().function(),
            self.device_id(),
            self.name(),
            self.vendor().name(),
        ))
    }
}
/// Scans the different pci devices connected to the system, and put them in a hashmap for easier use
pub fn init() {
    let buses = get_pci_buses();
    let mut devices = HashMap::with_capacity(buses.len());
    for pci_device in buses.iter().flat_map(|b| return b.devices.iter()) {
        if let Some(device) =
            pci_ids::Device::from_vid_pid(pci_device.vendor_id, pci_device.device_id)
        {
            let class = Classes::iter().find(|class| pci_device.class == class.id());
            if class.is_none() {
                continue;
            }
            let class = class.unwrap();
            devices.insert(
                pci_device.location,
                PciDevice {
                    raw: pci_device,
                    identified: device,
                    class,
                },
            );
        } else if pci_device.vendor_id == 0x1234 && pci_device.device_id == 0x1111 {
            // VGA QEMU pci device, not on pci ids, but we can safely skip it
        } else {
            log::error!("Unknown device: {:?}", pci_device);
        }
    }

    unsafe { MANAGER.replace(devices); }
}

#[repr(u8)]
pub enum PciCapability {
    Msi = 0x05,
    Msix = 0x11,
}

/// If a BAR's bits [2:1] equal this value, that BAR describes a 64-bit address.
/// If not, that BAR describes a 32-bit address.
const BAR_ADDRESS_IS_64_BIT: u32 = 2;

/// There is a maximum of 256 PCI buses on one system.
const MAX_PCI_BUSES: u16 = 256;
/// There is a maximum of 32 slots on one PCI bus.
const MAX_SLOTS_PER_BUS: u8 = 32;
/// There is a maximum of 32 functions (devices) on one PCI slot.
const MAX_FUNCTIONS_PER_SLOT: u8 = 8;

/// Addresses/offsets into the PCI configuration space should clear the least-significant 2 bits.
const PCI_CONFIG_ADDRESS_OFFSET_MASK: u8 = 0xFC;
const CONFIG_ADDRESS: u16 = 0xCF8;
const CONFIG_DATA: u16 = 0xCFC;

/// This port is used to specify the address in the PCI configuration space
/// for the next read/write of the `PCI_CONFIG_DATA_PORT`.
static PCI_CONFIG_ADDRESS_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_ADDRESS));

/// This port is used to transfer data to or from the PCI configuration space
/// specified by a previous write to the `PCI_CONFIG_ADDRESS_PORT`.
static PCI_CONFIG_DATA_PORT: Mutex<Port<u32>> = Mutex::new(Port::new(CONFIG_DATA));

#[derive(Debug)]
pub enum InterruptPin {
    A,
    B,
    C,
    D,
}

/// Returns a list of all PCI buses in this system.
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
fn get_pci_buses() -> &'static Vec<PciBus> {
    static PCI_BUSES: Once<Vec<PciBus>> = Once::new();
    return PCI_BUSES.call_once(scan_pci)
}

/// Returns a reference to the `RawPciDevice` with the given bus, slot, func identifier.
/// If the PCI bus hasn't been initialized, this initializes the PCI bus & scans it to enumerates devices.
fn get_pci_device_bsf(bus: u8, slot: u8, func: u8) -> Option<&'static RawPciDevice> {
    for b in get_pci_buses() {
        if b.bus_number == bus {
            for d in &b.devices {
                if d.slot == slot && d.func == func {
                    return Some(d);
                }
            }
        }
    }
    None
}

/// A PCI bus, which contains a list of PCI devices on that bus.
#[derive(Debug)]
pub struct PciBus {
    /// The number identifier of this PCI bus.
    pub bus_number: u8,
    /// The list of devices attached to this PCI bus.
    pub devices: Vec<RawPciDevice>,
}

/// Scans all PCI Buses (brute force iteration) to enumerate PCI Devices on each bus.
/// Initializes structures containing this information.
fn scan_pci() -> Vec<PciBus> {
    let mut buses: Vec<PciBus> = Vec::new();

    for bus in 0..MAX_PCI_BUSES {
        let bus = bus as u8;
        let mut device_list: Vec<RawPciDevice> = Vec::new();

        for slot in 0..MAX_SLOTS_PER_BUS {
            let loc_zero = PciLocation { bus, slot, func: 0 };
            // skip the whole slot if the vendor ID is 0xFFFF
            if 0xFFFF == loc_zero.pci_read_16(PCI_VENDOR_ID) {
                continue;
            }

            // If the header's MSB is set, then there are multiple functions for this device,
            // and we should check all 8 of them to be sure.
            // Otherwise, we only need to check the first function, because it's a single-function device.
            let header_type = loc_zero.pci_read_8(PCI_HEADER_TYPE);
            let functions_to_check = if header_type & 0x80 == 0x80 {
                0..MAX_FUNCTIONS_PER_SLOT
            } else {
                0..1
            };

            for f in functions_to_check {
                let location = PciLocation { bus, slot, func: f };
                let vendor_id = location.pci_read_16(PCI_VENDOR_ID);
                if vendor_id == 0xFFFF {
                    continue;
                }

                let device = RawPciDevice {
                    vendor_id,
                    device_id: location.pci_read_16(PCI_DEVICE_ID),
                    command: location.pci_read_16(PCI_COMMAND),
                    status: location.pci_read_16(PCI_STATUS),
                    revision_id: location.pci_read_8(PCI_REVISION_ID),
                    prog_if: location.pci_read_8(PCI_PROG_IF),
                    subclass: location.pci_read_8(PCI_SUBCLASS),
                    class: location.pci_read_8(PCI_CLASS),
                    cache_line_size: location.pci_read_8(PCI_CACHE_LINE_SIZE),
                    latency_timer: location.pci_read_8(PCI_LATENCY_TIMER),
                    header_type: location.pci_read_8(PCI_HEADER_TYPE),
                    bist: location.pci_read_8(PCI_BIST),
                    bars: [
                        location.pci_read_32(PCI_BAR0),
                        location.pci_read_32(PCI_BAR1),
                        location.pci_read_32(PCI_BAR2),
                        location.pci_read_32(PCI_BAR3),
                        location.pci_read_32(PCI_BAR4),
                        location.pci_read_32(PCI_BAR5),
                    ],
                    int_pin: location.pci_read_8(PCI_INTERRUPT_PIN),
                    int_line: location.pci_read_8(PCI_INTERRUPT_LINE),
                    location,
                };

                device_list.push(device);
            }
        }

        if !device_list.is_empty() {
            buses.push(PciBus {
                bus_number: bus,
                devices: device_list,
            });
        }
    }

    buses
}

/// The bus, slot, and function number of a given PCI device.
/// This offers methods for reading and writing the PCI config space.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PciLocation {
    pub bus: u8,
    pub slot: u8,
    pub func: u8,
}

impl PciLocation {
    #[must_use] pub fn bus(&self) -> u8 {
        self.bus
    }
    #[must_use] pub fn slot(&self) -> u8 {
        self.slot
    }
    #[must_use] pub fn function(&self) -> u8 {
        self.func
    }

    /// Computes a PCI address from bus, slot, func, and offset.
    /// The least two significant bits of offset are masked, so it's 4-byte aligned addressing,
    /// which makes sense since we read PCI data (from the configuration space) in 32-bit chunks.
    fn pci_address(self, offset: u8) -> u32 {
        (u32::from(self.bus) << 16)
            | (u32::from(self.slot) << 11)
            | (u32::from(self.func) << 8)
            | (u32::from(offset) & u32::from(PCI_CONFIG_ADDRESS_OFFSET_MASK))
            | 0x8000_0000
    }

    /// read 32-bit data at the specified `offset` from the PCI device specified by the given `bus`, `slot`, `func` set.
    pub fn pci_read_32(&self, offset: u8) -> u32 {
        unsafe {
            PCI_CONFIG_ADDRESS_PORT
                .lock()
                .write(self.pci_address(offset));
        };
        Self::read_data_port() >> ((offset & (!PCI_CONFIG_ADDRESS_OFFSET_MASK)) * 8)
    }

    /// Read 16-bit data at the specified `offset` from this PCI device.
    #[must_use] pub fn pci_read_16(&self, offset: u8) -> u16 {
        self.pci_read_32(offset) as u16
    }

    /// Read 8-bit data at the specified `offset` from the PCI device.
    #[must_use] pub fn pci_read_8(&self, offset: u8) -> u8 {
        self.pci_read_32(offset) as u8
    }

    /// Write 32-bit data to the specified `offset` for the PCI device.
    pub fn pci_write(&self, offset: u8, value: u32) {
        unsafe {
            PCI_CONFIG_ADDRESS_PORT
                .lock()
                .write(self.pci_address(offset));
            Self::write_data_port((value) << ((offset & 2) * 8));
        }
    }

    fn write_data_port(value: u32) {
        unsafe {
            PCI_CONFIG_DATA_PORT.lock().write(value);
        }
    }

    pub fn read_data_port() -> u32 {
        unsafe { return PCI_CONFIG_DATA_PORT.lock().read() }
    }

    /// Sets the PCI device's bit 3 in the command portion, which is apparently needed to activate DMA (??)
    pub fn pci_set_command_bus_master_bit(&self) {
        unsafe {
            PCI_CONFIG_ADDRESS_PORT
                .lock()
                .write(self.pci_address(PCI_COMMAND));
        };
        let inval = Self::read_data_port();
        serial_println!(
            "pci_set_command_bus_master_bit: RawPciDevice: {}, read value: {:#x}",
            self,
            inval
        );
        Self::write_data_port(inval | (1 << 2));
        serial_println!(
            "pci_set_command_bus_master_bit: RawPciDevice: {}, read value AFTER WRITE CMD: {:#x}",
            self,
            Self::read_data_port()
        );
    }

    /// Sets the PCI device's command bit 10 to disable legacy interrupts
    pub fn pci_set_interrupt_disable_bit(&self) {
        unsafe {
            PCI_CONFIG_ADDRESS_PORT
                .lock()
                .write(self.pci_address(PCI_COMMAND));
        };
        let command = Self::read_data_port();
        serial_println!(
            "pci_set_interrupt_disable_bit: RawPciDevice: {}, read value: {:#x}",
            self,
            command
        );
        const INTERRUPT_DISABLE: u32 = 1 << 10;
        Self::write_data_port(command | INTERRUPT_DISABLE);
        serial_println!(
            "pci_set_interrupt_disable_bit: RawPciDevice: {} read value AFTER WRITE CMD: {:#x}",
            self,
            Self::read_data_port()
        );
    }

    /// Explores the PCI config space and returns address of requested capability, if present.
    /// PCI capabilities are stored as a linked list in the PCI config space,
    /// with each capability storing the pointer to the next capability right after its ID.
    /// The function returns a None value if capabilities are not valid for this device
    /// or if the requested capability is not present.
    fn find_pci_capability(&self, pci_capability: PciCapability) -> Option<u8> {
        let pci_capability = pci_capability as u8;
        let status = self.pci_read_16(PCI_STATUS);

        // capabilities are only valid if bit 4 of status register is set
        const CAPABILITIES_VALID: u16 = 1 << 4;
        if status & CAPABILITIES_VALID != 0 {
            // retrieve the capabilities pointer from the pci config space
            let capabilities = self.pci_read_8(PCI_CAPABILITIES);
            // debug!("capabilities pointer: {:#X}", capabilities);

            // mask the bottom 2 bits of the capabilities pointer to find the address of the first capability
            let mut cap_addr = capabilities & 0xFC;

            // the last capability will have its next pointer equal to zero
            let final_capability = 0;

            // iterate through the linked list of capabilities until the requested capability is found or the list reaches its end
            while cap_addr != final_capability {
                // the capability header is a 16 bit value which contains the current capability ID and the pointer to the next capability
                let cap_header = self.pci_read_16(cap_addr);

                // the id is the lower byte of the header
                let cap_id = (cap_header & 0xFF) as u8;

                if cap_id == pci_capability {
                    serial_println!("Found capability: {:#X} at {:#X}", pci_capability, cap_addr);
                    return Some(cap_addr);
                }

                // find address of next capability which is the higher byte of the header
                cap_addr = ((cap_header >> 8) & 0xFF) as u8;
            }
        }
        None
    }
}

impl fmt::Display for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "b{}.s{}.f{}", self.bus, self.slot, self.func)
    }
}

impl fmt::Debug for PciLocation {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{self}")
    }
}

/// Contains information common to every type of PCI Device,
/// and offers functions for reading/writing to the PCI configuration space.
///
/// For more, see [this partial table](http://wiki.osdev.org/PCI#Class_Codes)
/// of `class`, `subclass`, and `prog_if` codes,
#[derive(Debug)]
pub struct RawPciDevice {
    /// the bus, slot, and function number that locates this PCI device in the bus tree.
    pub location: PciLocation,

    /// The class code, used to determine device type.
    pub class: u8,
    /// The subclass code, used to determine device type.
    pub subclass: u8,
    /// The programming interface of this PCI device, also used to determine device type.
    pub prog_if: u8,
    /// The six Base Address Registers (BARs)
    pub bars: [u32; 6],
    pub vendor_id: u16,
    pub device_id: u16,
    pub command: u16,
    pub status: u16,
    pub revision_id: u8,
    pub cache_line_size: u8,
    pub latency_timer: u8,
    pub header_type: u8,
    pub bist: u8,
    pub int_pin: u8,
    pub int_line: u8,
}

impl RawPciDevice {
    /// Returns the base address of the memory region specified by the given `BAR`
    /// (Base Address Register) for this PCI device.
    ///
    /// # Argument
    /// * `bar_index` must be between `0` and `5` inclusively, as each PCI device
    ///   can only have 6 BARs at the most.  
    ///
    /// Note that if the given `BAR` actually indicates it is part of a 64-bit address,
    /// it will be used together with the BAR right above it (`bar + 1`), e.g., `BAR1:BAR0`.
    /// If it is a 32-bit address, then only the given `BAR` will be accessed.
    pub fn determine_mem_base(&self, bar_index: usize) -> Result<PciMemoryBase, &'static str> {
        let mut bar = if let Some(bar_value) = self.bars.get(bar_index) {
            *bar_value
        } else {
            return Err("BAR index must be between 0 and 5 inclusive");
        };
        if bar.get_bit(0) {
            let base = bar.get_bits(2..);
            Ok(PciMemoryBase::IOSpace(base))
        } else {
            // Check bits [2:1] of the bar to determine address length (64-bit or 32-bit)
            let base = if bar.get_bits(1..=2) == BAR_ADDRESS_IS_64_BIT {
                // Here: this BAR is the lower 32-bit part of a 64-bit address,
                // so we need to access the next highest BAR to get the address's upper 32 bits.
                let next_bar = *self
                    .bars
                    .get(bar_index + 1)
                    .ok_or("next highest BAR index is out of range")?;
                // Clear the bottom 4 bits because it's a 16-byte aligned address

                (*bar.set_bits(0..4, 0) as usize | ((next_bar as usize) << 32))
                // .ok_or("determine_mem_base(): [64-bit] BAR physical address was invalid")?
            } else {
                // Here: this BAR is the lower 32-bit part of a 64-bit address,
                // so we need to access the next highest BAR to get the address's upper 32 bits.
                // Also, clear the bottom 4 bits because it's a 16-byte aligned address.
                (*bar.set_bits(0..4, 0) as usize)
                // .ok_or("determine_mem_base(): [32-bit] BAR physical address was invalid")?
            }
            .try_into()
            .unwrap();
            Ok(PciMemoryBase::MemorySpace(PhysAddr::new(base)))
        }
    }

    /// Returns the size in bytes of the memory region specified by the given `BAR`
    /// (Base Address Register) for this PCI device.
    ///
    /// # Argument
    /// * `bar_index` must be between `0` and `5` inclusively, as each PCI device
    /// can only have 6 BARs at the most.
    ///
    #[must_use] pub fn determine_mem_size(&self, bar_index: usize) -> u32 {
        assert!(bar_index < 6);
        // Here's what we do:
        // (1) Write all `1`s to the specified BAR
        // (2) Read that BAR value again
        // (3) Mask the info bits (bits [3:0]) of the BAR value read in Step 2
        // (4) Bitwise "not" (negate) that value, then add 1.
        //     The resulting value is the size of that BAR's memory region.
        // (5) Restore the original value to that BAR
        let bar_offset = PCI_BAR0 + (bar_index as u8 * 4);
        let original_value = self.bars[bar_index];

        self.pci_write(bar_offset, 0xFFFF_FFFF); // Step 1
        let mut mem_size = self.pci_read_32(bar_offset); // Step 2
        mem_size.set_bits(0..4, 0); // Step 3
        mem_size = !(mem_size); // Step 4
        mem_size += 1; // Step 4
        self.pci_write(bar_offset, original_value); // Step 5
        mem_size
    }

    /// Enable MSI interrupts for a PCI device.
    /// We assume the device only supports one MSI vector
    /// and set the interrupt number and core id for that vector.
    /// If the MSI capability is not supported then an error message is returned.
    ///
    /// # Arguments
    /// * `core_id`: core that interrupt will be routed to
    /// * `int_num`: interrupt number to assign to the MSI vector
    pub fn pci_enable_msi(&self, core_id: u8, int_num: u8) -> Result<(), &'static str> {
        // find out if the device is msi capable
        let cap_addr = self
            .find_pci_capability(PciCapability::Msi)
            .ok_or("Device not MSI capable")?;

        // offset in the capability space where the message address register is located
        const MESSAGE_ADDRESS_REGISTER_OFFSET: u8 = 4;
        // the memory region is a constant defined for Intel cpus where MSI messages are written
        // it should be written to bit 20 of the message address register
        const MEMORY_REGION: u32 = 0x0FEE << 20;
        // the core id tells which cpu the interrupt will be routed to
        // it should be written to bit 12 of the message address register
        let core = u32::from(core_id) << 12;
        // set the core the MSI will be sent to in the Message Address Register (Intel Arch SDM, vol3, 10.11)
        self.pci_write(
            cap_addr + MESSAGE_ADDRESS_REGISTER_OFFSET,
            MEMORY_REGION | core,
        );

        // offset in the capability space where the message data register is located
        const MESSAGE_DATA_REGISTER_OFFSET: u8 = 12;
        // Set the interrupt number for the MSI in the Message Data Register
        self.pci_write(cap_addr + MESSAGE_DATA_REGISTER_OFFSET, u32::from(int_num));

        // offset in the capability space where the message control register is located
        const MESSAGE_CONTROL_REGISTER_OFFSET: u8 = 2;
        // to enable the MSI capability, we need to set it bit 0 of the message control register
        const MSI_ENABLE: u32 = 1;
        let ctrl = u32::from(self.pci_read_16(cap_addr + MESSAGE_CONTROL_REGISTER_OFFSET));
        // enable MSI in the Message Control Register
        self.pci_write(
            cap_addr + MESSAGE_CONTROL_REGISTER_OFFSET,
            ctrl | MSI_ENABLE,
        );

        Ok(())
    }

    /// Enable MSI-X interrupts for a PCI device.
    /// Only the enable bit is set and the remaining initialization steps of
    /// setting the interrupt number and core id should be completed in the device driver.
    pub fn pci_enable_msix(&self) -> Result<(), &'static str> {
        // find out if the device is msi-x capable
        let cap_addr = self
            .find_pci_capability(PciCapability::Msix)
            .ok_or("Device not MSI-X capable")?;

        // offset in the capability space where the message control register is located
        const MESSAGE_CONTROL_REGISTER_OFFSET: u8 = 2;
        let ctrl = u32::from(self.pci_read_16(cap_addr + MESSAGE_CONTROL_REGISTER_OFFSET));

        // write to bit 15 of Message Control Register to enable MSI-X
        const MSIX_ENABLE: u32 = 1 << 15;
        self.pci_write(
            cap_addr + MESSAGE_CONTROL_REGISTER_OFFSET,
            ctrl | MSIX_ENABLE,
        );

        // let ctrl = pci_read_32(dev.bus, dev.slot, dev.func, cap_addr);
        // debug!("MSIX HEADER AFTER ENABLE: {:#X}", ctrl);

        Ok(())
    }

    /// Returns the memory mapped msix vector table
    ///
    /// - returns `Err("Device not MSI-X capable")` if the device doesn't have the MSI-X capability
    /// - returns `Err("Invalid BAR content")` if the Base Address Register contains an invalid address
    // pub fn pci_mem_map_msix(&self, max_vectors: usize) -> Result<MsixVectorTable, &'static str> {
    //     // retreive the address in the pci config space for the msi-x capability
    //     let cap_addr = self.find_pci_capability(PciCapability::Msix).ok_or("Device not MSI-X capable")?;
    //     // find the BAR used for msi-x
    //     let vector_table_offset = 4;
    //     let table_offset = self.pci_read_32(cap_addr + vector_table_offset);
    //     let bar = table_offset & 0x7;
    //     let offset = table_offset >> 3;
    //     // find the memory base address and size of the area for the vector table
    //     let mem_base = PhysAddr::new((self.bars[bar as usize] + offset) as usize)
    //         .ok_or("Invalid BAR content")?;
    //     let mem_size_in_bytes = core::mem::size_of::<MsixVectorEntry>() * max_vectors;

    //     // debug!("msi-x vector table bar: {}, base_address: {:#X} and size: {} bytes", bar, mem_base, mem_size_in_bytes);

    //     let msix_mapped_pages = map_frame_range(mem_base, mem_size_in_bytes, MMIO_FLAGS)?;
    //     let vector_table = BorrowedSliceMappedPages::from_mut(msix_mapped_pages, 0, max_vectors)
    //         .map_err(|(_mp, err)| err)?;

    //     Ok(MsixVectorTable::new(vector_table))
    // }

    // /// Maps device memory specified by a Base Address Register.
    // ///
    // /// # Arguments
    // /// * `bar_index`: index of the Base Address Register to use
    // pub fn pci_map_bar_mem(&self, bar_index: usize) -> Result<MappedPages, &'static str> {
    //     let mem_base = self.determine_mem_base(bar_index)?;
    //     let mem_size = self.determine_mem_size(bar_index);
    //     map_frame_range(mem_base, mem_size as usize, MMIO_FLAGS)
    // }

    /// Reads and returns this PCI device's interrupt line and interrupt pin registers.
    ///
    /// Returns an error if this PCI device's interrupt pin value is invalid (greater than 4).
    pub fn pci_get_interrupt_info(
        &self,
    ) -> Result<(Option<u8>, Option<InterruptPin>), &'static str> {
        let int_line = match self.pci_read_8(PCI_INTERRUPT_LINE) {
            0xff => None,
            other => Some(other),
        };

        let int_pin = match self.pci_read_8(PCI_INTERRUPT_PIN) {
            0 => None,
            1 => Some(InterruptPin::A),
            2 => Some(InterruptPin::B),
            3 => Some(InterruptPin::C),
            4 => Some(InterruptPin::D),
            _ => return Err("pci_get_interrupt_info: Invalid Register Value for Interrupt Pin"),
        };

        Ok((int_line, int_pin))
    }
}

#[derive(Debug)]
pub enum PciMemoryBase {
    MemorySpace(PhysAddr),
    IOSpace(u32),
}
impl PciMemoryBase {
    #[must_use] pub fn as_u64(&self) -> u64 {
        match self {
            PciMemoryBase::MemorySpace(m) => m.as_u64(),
            PciMemoryBase::IOSpace(io) => u64::from(*io),
        }
    }
}

impl Deref for RawPciDevice {
    type Target = PciLocation;
    fn deref(&self) -> &PciLocation {
        &self.location
    }
}
impl DerefMut for RawPciDevice {
    fn deref_mut(&mut self) -> &mut PciLocation {
        &mut self.location
    }
}

/// Lists the 2 possible PCI configuration space access mechanisms
/// that can be found from the LSB of the devices's BAR0
pub enum PciConfigSpaceAccessMechanism {
    MemoryMapped = 0,
    IoPort = 1,
}

// A memory-mapped array of [`MsixVectorEntry`]
// pub struct MsixVectorTable {
//     entries: BorrowedSliceMappedPages<MsixVectorEntry, Mutable>,
// }

// impl MsixVectorTable {
//     pub fn new(entries: BorrowedSliceMappedPages<MsixVectorEntry, Mutable>) -> Self {
//         Self { entries }
//     }
// }
// impl Deref for MsixVectorTable {
//     type Target = [MsixVectorEntry];
//     fn deref(&self) -> &Self::Target {
//         &self.entries
//     }
// }
// impl DerefMut for MsixVectorTable {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.entries
//     }
// }

// A single Message Signaled Interrupt entry.
//
// This entry contains the interrupt's IRQ vector number
// and the CPU to which the interrupt will be delivered.
// #[derive(FromBytes)]
// #[repr(C)]
// pub struct MsixVectorEntry {
//     /// The lower portion of the address for the memory write transaction.
//     /// This part contains the CPU ID which the interrupt will be redirected to.
//     msg_lower_addr:         Volatile<u32>,
//     /// The upper portion of the address for the memory write transaction.
//     msg_upper_addr:         Volatile<u32>,
//     /// The data portion of the msi vector which contains the interrupt number.
//     msg_data:               Volatile<u32>,
//     /// The control portion which contains the interrupt mask bit.
//     vector_control:         Volatile<u32>,
// }

// impl MsixVectorEntry {
//     /// Sets interrupt destination & number for this entry and makes sure the
//     /// interrupt is unmasked (PCI Controller side).
//     pub fn init(&mut self, cpu_id: CpuId, int_num: InterruptNumber) {
//         // unmask the interrupt
//         self.vector_control.write(MSIX_UNMASK_INT);
//         let lower_addr = self.msg_lower_addr.read();

//         // set the CPU to which this interrupt will be delivered.
//         let dest_id = (cpu_id.into_u8() as u32) << MSIX_DEST_ID_SHIFT;
//         let address = lower_addr & !MSIX_ADDRESS_BITS;
//         self.msg_lower_addr.write(address | MSIX_INTERRUPT_REGION | dest_id);

//         // write interrupt number
//         self.msg_data.write(int_num as u32);

//         if false {
//             let control = self.vector_control.read();
//             debug!("Created MSI vector: control: {}, CPU: {}, int: {}", control, cpu_id, int_num);
//         }
//     }
// }

// /// A constant which indicates the region that is reserved for interrupt messages
// const MSIX_INTERRUPT_REGION:    u32 = 0xFEE << 20;
// /// The location in the lower address register where the destination CPU ID is written
// const MSIX_DEST_ID_SHIFT:       u32 = 12;
// /// The bits in the lower address register that need to be cleared and set
// const MSIX_ADDRESS_BITS:        u32 = 0xFFFF_FFF0;
// /// Clear the vector control field to unmask the interrupt
// const MSIX_UNMASK_INT:          u32 = 0;

// Basically the crates 'pci' and 'pci_ids' are kinda bad, so I'm making a wrapper around both
// Maybe make it a entire new library named 'pci_no_std' that we publish on crates.io ?
// We'll see
pub struct Device {
    class: &'static Class,
    device: &'static pci_ids::Device,
    pub bus_device: RawPciDevice,
}

impl Device {
    #[must_use] pub fn at_bus(bus: u8) -> Vec<Self> {
        let mut devices: Vec<Self> = Vec::new();
        for slot in 0..MAX_SLOTS_PER_BUS {
            let loc_zero = PciLocation { bus, slot, func: 0 };
            // skip the whole slot if the vendor ID is 0xFFFF
            if 0xFFFF == loc_zero.pci_read_16(PCI_VENDOR_ID) {
                continue;
            }

            // If the header's MSB is set, then there are multiple functions for this device,
            // and we should check all 8 of them to be sure.
            // Otherwise, we only need to check the first function, because it's a single-function device.
            let header_type = loc_zero.pci_read_8(PCI_HEADER_TYPE);
            let functions_to_check = if header_type & 0x80 == 0x80 {
                0..MAX_FUNCTIONS_PER_SLOT // Were is c func ?
            } else {
                0..1
            };

            for f in functions_to_check {
                let location = PciLocation { bus, slot, func: f };
                let vendor_id = location.pci_read_16(PCI_VENDOR_ID);
                if vendor_id == 0xFFFF {
                    continue;
                }

                let device = RawPciDevice {
                    vendor_id,
                    device_id: location.pci_read_16(PCI_DEVICE_ID),
                    command: location.pci_read_16(PCI_COMMAND),
                    status: location.pci_read_16(PCI_STATUS),
                    revision_id: location.pci_read_8(PCI_REVISION_ID),
                    prog_if: location.pci_read_8(PCI_PROG_IF),
                    subclass: location.pci_read_8(PCI_SUBCLASS),
                    class: location.pci_read_8(PCI_CLASS),
                    cache_line_size: location.pci_read_8(PCI_CACHE_LINE_SIZE),
                    latency_timer: location.pci_read_8(PCI_LATENCY_TIMER),
                    header_type: location.pci_read_8(PCI_HEADER_TYPE),
                    bist: location.pci_read_8(PCI_BIST),
                    bars: [
                        location.pci_read_32(PCI_BAR0),
                        location.pci_read_32(PCI_BAR1),
                        location.pci_read_32(PCI_BAR2),
                        location.pci_read_32(PCI_BAR3),
                        location.pci_read_32(PCI_BAR4),
                        location.pci_read_32(PCI_BAR5),
                    ],
                    int_pin: location.pci_read_8(PCI_INTERRUPT_PIN),
                    int_line: location.pci_read_8(PCI_INTERRUPT_LINE),
                    location,
                };

                let device = Self {
                    class: pci_ids::Class::from_id(device.class).unwrap(),
                    device: pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id)
                        .unwrap(),
                    bus_device: device,
                };

                devices.push(device);
            }
        }
        devices
    }
    #[must_use] pub fn product_name(&self) -> &str {
        self.device.name()
    }
    #[must_use] pub fn product_id(&self) -> u16 {
        self.device.id()
    }
    #[must_use] pub fn vendor_name(&self) -> &str {
        self.device.vendor().name()
    }
    #[must_use] pub fn vendor_id(&self) -> u16 {
        self.device.vendor().id()
    }
    #[must_use] pub fn class_id(&self) -> u8 {
        self.class.id()
    }
    #[must_use] pub fn class_name(&self) -> &str {
        self.class.name()
    }
}
