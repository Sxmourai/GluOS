use bit_field::BitField;
use x86_64::instructions::port::{PortGeneric, PortReadOnly, PortWriteOnly, ReadWriteAccess};

use crate::{bit_manipulation::outb, dbg, descriptor_tables};

pub const DATA_PORT: PortGeneric<u8, ReadWriteAccess> = PortGeneric::new(0x60);
pub const STATUS_REG: PortReadOnly<u8> = PortReadOnly::new(0x64);
pub const COMMAND_REG: PortWriteOnly<u8> = PortWriteOnly::new(0x64);

/// false = empty, true = full
/// (must be set before attempting to read data from IO port 0x60)
pub const STATUS_OUTPUT_BUFFER: u8 = 1 << 0;
/// false = empty, true = full
/// must be clear before attempting to write data to IO port 0x60 or IO port 0x64
pub const STATUS_INPUT_BUFFER: u8 = 1 << 1;
/// Meant to be cleared on reset and set by firmware (via. PS/2 Controller Configuration Byte) if the system passes self tests (POST)
pub const STATUS_SYSTEM_FLAG: u8 = 1 << 2;
/// (false = data written to input buffer is data for PS/2 device, true = data written to input buffer is data for PS/2 controller command
pub const STATUS_COMMAND_OR_DATA: u8 = 1 << 3;
/// May be "keyboard lock" (more likely unused on modern systems)
pub const STATUS_UNKNOWN: u8 = 1 << 4;
/// May be "receive time-out" or "second PS/2 port output buffer full"
pub const STATUS_UNKNOWN2: u8 = 1 << 5;
/// (false = no error, true = time-out error)
pub const STATUS_TIMEOUT: u8 = 1 << 6;
/// (false = no error, true = parity error)
pub const STATUS_PARITY: u8 = 1 << 7;

// Simple utility to turn on/off error messages when assert_ack isn't equals to ack
const ACK_LOGGING: bool = false;

/// Initialises & resets the ps2 controller
/// Ps2 controller is used for keyboard input and mouse.
/// Currently we don't have a great driver, but it works under QEMU
/// TODO Indentify the different devices on the ps2 controller
pub async fn init() {
    Ps2Controller::init().unwrap();
}

pub struct Ps2Controller {
    available_channels: [bool; 2],
}
impl Ps2Controller {
    /// Tries to init Ps2Controller, returns option because we can't do much error handling
    pub fn init() -> Option<Self> {
        match descriptor_tables!().version() {
            crate::acpi::tables::AcpiVersion::One => {}
            crate::acpi::tables::AcpiVersion::Two => {
                if descriptor_tables!().fadt.boot_architecture_flags & 2 == 0 {
                    log::error!("No configurable Ps2 controller doesn't exist !");
                    return None;
                }
            }
        }
        // Disable first PS/2 port, so that no ps2 device interfer with us
        Self::send_command(0xAD);
        // Disable second PS/2 port, so that no ps2 device interfer with us
        Self::send_command(0xA7);
        // Flush output buffer
        //TODO This code shares a lot of code with poll_bit, should put in common
        for i in 0..100_000 {
            #[allow(const_item_mutation)]
            let _ = unsafe { DATA_PORT.read() };
            #[allow(const_item_mutation)]
            if unsafe { STATUS_REG.read() } & 1 == 1 {
                break;
            }
        }
        // Set controller config byte
        Self::send_command(0x20);
        let mut config_byte = Self::retrieve_command().unwrap(); // Read controller config byte
        config_byte.set_bit(0, false);
        config_byte.set_bit(1, false);
        config_byte.set_bit(6, false);
        /// If false we can be sure there's no dual channel
        let mut available_channels = [true; 2];
        available_channels[1] = config_byte.get_bit(5);
        Self::send_command_next(0x60, config_byte); // Rewrite controller config_byte

        // Perform Controller Self Test
        Self::send_command(0xAA);
        if Self::retrieve_command().unwrap() != 0x55 {
            log::error!("Failed perfoming self test on ps/2 controller !");
            return None;
        }
        /// Even though dual_channel is set, we aren't sure it's dual
        if available_channels[1] {
            // Determine If There Are 2 Channels
            // We should skip this step if we saw it only has one channel from upper ^^ config_byte.get_bit(5)
            Self::send_command(0xA8); // Enable second ps/2 controller
            Self::send_command(0x20); // Read config byte
            let config_byte = Self::retrieve_command().unwrap(); // Read controller config byte
            if config_byte.get_bit(5) == true {
                // Not dual channel... We had hope tho
                available_channels[1] = false;
            } else {
                // It's dual channel !
                Self::send_command(0xA7); // Re disables second ps/2 controller
            }
        }

        // Perform Interface Tests
        log::trace!("Performing interface tests");
        Self::send_command(0xAB); // Test first PS/2 Port
        if Self::retrieve_command().unwrap() != 0x0 {
            //TODO 0x01 clock line stuck low 0x02 clock line stuck high 0x03 data line stuck low 0x04 data line stuck high
            log::error!("Failed testing first PS/2 port !");
            if available_channels[1] == false {
                // No port available
                return None;
            }
        }
        if available_channels[1] {
            Self::send_command(0xA9);
            //TODO 0x01 clock line stuck low 0x02 clock line stuck high 0x03 data line stuck low 0x04 data line stuck high
            if Self::retrieve_command().unwrap() != 0x0 {
                log::error!("Failed testing second PS/2 port !");
                return None;
            }
        }

        // Step 9:Enable devices
        log::trace!("Enabling ports");
        Self::send_command(0xAE);
        if available_channels[1] {
            Self::send_command(0xA8); // Second port
        }
        //Enable IRQ
        log::trace!("Enabling IRQ's");
        Self::send_command(0x20);
        let mut config_byte = Self::retrieve_command().unwrap();
        config_byte.set_bit(0, true);
        if available_channels[1] {
            config_byte.set_bit(1, true);
        }
        Self::send_command_next(0x60, config_byte);

        // Step 10: Reset devices
        log::trace!("Resetting devices");
        Self::device_send_data_first(0xFF);
        if Self::assert_ack() {
            log::trace!("First port reseted !")
        }
        if available_channels[1] {
            Self::device_send_data_second(0xFF);
            if Self::assert_ack() {
                dbg!(1);
                log::trace!("Second port reseted !")
            }
        }
        // Identify devices
        log::trace!("Identifying devices");
        // Doesn't do anything for now, but if remove it the mouse driver doesn't work
        Self::send_command(0xF5); // Disable scanning to not have trash on line
        (Self::assert_ack());
        Self::device_send_data_first(0xF2); // Send identify
        (Self::assert_ack());
        for i in 0..=2 {
            Self::retrieve_command().unwrap();
        }
        Self::device_send_data_first(0xF4); // Re Enable scanning
        (Self::assert_ack());

        // Self::device_send_data_second(0xF5); // Disable scanning to not have trash on line
        // assert!(Self::assert_ack());
        // Self::device_send_data_second(0xF2); // Send identify
        // assert!(Self::assert_ack());
        // for i in 0..=2 {
        //     let b = Self::retrieve_command();
        //     if b.is_ok() {
        //         dbg!(b.unwrap());
        //     }
        // }
        // Self::device_send_data_second(0xF4); // Re Enable scanning
        // assert!(Self::assert_ack());

        Some(Self { available_channels })
    }
    /// Sends a byte to the first port
    pub fn device_send_data_first(data: u8) {
        Self::poll_bit(STATUS_INPUT_BUFFER).unwrap();
        #[allow(const_item_mutation)]
        unsafe {
            DATA_PORT.write(data)
        }
    }
    /// Sends a byte to the second port
    pub fn device_send_data_second(data: u8) {
        Self::send_command(0xD4); // Select second port
        Self::poll_bit(STATUS_INPUT_BUFFER).unwrap();
        #[allow(const_item_mutation)]
        unsafe {
            DATA_PORT.write(data)
        }
    }

    pub fn send_command(command: u8) {
        #[allow(const_item_mutation)]
        unsafe {
            COMMAND_REG.write(command);
        };
    }
    pub fn read_data() -> u8 {
        #[allow(const_item_mutation)]
        unsafe {
            DATA_PORT.read()
        }
    }
    /// Sends a 2 bytes command to Ps2 controller
    pub fn send_command_next(fst_command: u8, next_byte: u8) {
        Self::send_command(fst_command);
        Self::poll_bit(STATUS_OUTPUT_BUFFER).unwrap();
        #[allow(const_item_mutation)]
        unsafe {
            DATA_PORT.write(next_byte)
        };
    }

    pub fn assert_ack() -> bool {
        let ack = Self::retrieve_command().unwrap() == 0xFA;
        if !ack && ACK_LOGGING {
            log::error!("Failed assert on ACK !");
        }
        ack
    }

    /// Reads status port until inb(status) & bit == 0
    /// If times out, return error
    pub fn poll_bit(bit: u8) -> Result<(), Ps2PollingError> {
        for i in 0..100000 {
            #[allow(const_item_mutation)]
            if unsafe { STATUS_REG.read() } & bit == 0 {
                return Ok(());
            }
        }
        Err(Ps2PollingError::TimeOut)
    }
    /// Reads status port until inb(status) & bit == bit
    /// If times out, return error
    pub fn poll_bit_set(bit: u8) -> Result<(), Ps2PollingError> {
        for i in 0..100000 {
            #[allow(const_item_mutation)]
            if unsafe { STATUS_REG.read() } & bit == bit {
                return Ok(());
            }
        }
        Err(Ps2PollingError::TimeOut)
    }
    /// Tries to retrieve the contents of a previously sent command
    /// TODO Make a proper safe wrapper around send/receiving commands
    pub fn retrieve_command() -> Result<u8, Ps2PollingError> {
        Self::poll_bit_set(STATUS_OUTPUT_BUFFER)?;
        // We know the output buffer status port has been polled, so the data has arrived
        #[allow(const_item_mutation)]
        Ok(unsafe { DATA_PORT.read() })
    }
}

#[derive(Debug)]
pub enum Ps2PollingError {
    TimeOut,
}
