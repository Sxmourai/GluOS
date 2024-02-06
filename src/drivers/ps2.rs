use bit_field::BitField;
use x86_64::instructions::port::{PortGeneric, PortReadOnly, PortWriteOnly, ReadWriteAccess};

use crate::{bit_manipulation::outb, dbg, descriptor_tables};

pub const DATA_PORT: PortGeneric<u8, ReadWriteAccess> = PortGeneric::new(0x60);
pub const STATUS_REG: PortReadOnly<u8> = PortReadOnly::new(0x64);
pub const COMMAND_REG: PortWriteOnly<u8> = PortWriteOnly::new(0x64);

/// false = empty, true = full
/// (must be set before attempting to read data from IO port 0x60)
pub const STATUS_OUTPUT_BUFFER:u8 = 1<<0;
/// false = empty, true = full
/// must be clear before attempting to write data to IO port 0x60 or IO port 0x64
pub const STATUS_INPUT_BUFFER: u8 = 1<<1;
/// Meant to be cleared on reset and set by firmware (via. PS/2 Controller Configuration Byte) if the system passes self tests (POST)
pub const STATUS_SYSTEM_FLAG: u8 = 1<<2;
/// (false = data written to input buffer is data for PS/2 device, true = data written to input buffer is data for PS/2 controller command
pub const STATUS_COMMAND_OR_DATA: u8 = 1<<3;
/// May be "keyboard lock" (more likely unused on modern systems)
pub const STATUS_UNKNOWN: u8 = 1<<4;
/// May be "receive time-out" or "second PS/2 port output buffer full"
pub const STATUS_UNKNOWN2: u8 = 1<<5;
/// (false = no error, true = time-out error)
pub const STATUS_TIMEOUT: u8 = 1<<6;
/// (false = no error, true = parity error)
pub const STATUS_PARITY: u8 = 1<<7;

/// Tries to init the Ps2 Controller, but should init USB devices BEFORE
pub fn init() {
    //TODO If no ACPI then there is a ps2 controller, we should handle the no ACPI case
    if descriptor_tables!().fadt.boot_architecture_flags&2==0 {
        // None on QEMU from my tests
        log::error!("Ps2 controller doesn't exist ! Should we continue ?");
        return
    }
    // Disable first PS/2 port, so that no ps2 device interfer with us
    command(0xAD).unwrap();
    // Disable second PS/2 port, so that no ps2 device interfer with us
    command(0xA7).unwrap();
    // Flush output buffer
    //TODO This code shares a lot of code with poll_bit, should put in common
    for i in 0..100_000 {
        #[allow(const_item_mutation)]
        let _ = unsafe{DATA_PORT.read()};
        #[allow(const_item_mutation)]
        if unsafe{STATUS_REG.read()}&1==1 {
            break
        }
    }
    // Set controller config byte
    let mut config_byte = command(0x20).unwrap(); // Read controller config byte
    config_byte.set_bit(0, false);
    config_byte.set_bit(1, false);
    config_byte.set_bit(6, false);
    let mut dual_channel = config_byte.get_bit(5);
    command_next(0x60, config_byte); // Rewrite controller config_byte

    // Perform Controller Self Test
    if command(0xAA).unwrap() != 0x55 {
        log::error!("Failed perfoming self test on ps/2 controller !");
        return
    }
    // Determine If There Are 2 Channels
    // We should skip this step if we saw it only has one channel from upper ^^ config_byte.get_bit(5)
    command(0xA8).unwrap(); // Enable second ps/2 controller
    let config_byte = command(0x20).unwrap(); // Read controller config byte
    if config_byte.get_bit(5)==true { // Should be false (false=enabled)
        dual_channel = false;
    } else { // If dual channel
        command(0xA7).unwrap(); // Re disables second ps/2 controller
    }

    // Perform Interface Tests
    dbg!(command(0xAB).unwrap());
    if dual_channel {
        dbg!(command(0xA9).unwrap());
    }
//TODO https://wiki.osdev.org/"8042"_PS/2_Controller
// When we find out how to tell qemu to make a ps2 controller
}

/// Reads status port until inb(status) & bit == bit
/// If times out, return error
pub fn poll_bit(bit: u8) -> Result<(), Ps2PollingError> {
    for i in 0..100000 {
        #[allow(const_item_mutation)]
        if unsafe{STATUS_REG.read()} & bit==bit {
            return Ok(())
        }
    }
    Err(Ps2PollingError::TimeOut)
}
/// Sends a command to Ps2 controller, and returns the response byte
pub fn command(command: u8) -> Result<u8, Ps2PollingError> {
    #[allow(const_item_mutation)]
    unsafe { COMMAND_REG.write(command) }; // Send command to COMMAND_REG
    poll_bit(STATUS_OUTPUT_BUFFER)?;
    // We know the output buffer status port has been polled, so the data has arrived
    #[allow(const_item_mutation)]
    Ok(unsafe{DATA_PORT.read()})
}
/// Sends a command to Ps2 controller, and returns the response byte
pub fn command_next(command: u8, next_byte: u8) -> Result<u8, Ps2PollingError> {
    #[allow(const_item_mutation)]
    unsafe { COMMAND_REG.write(command) }; // Send command to COMMAND_REG
    poll_bit(STATUS_OUTPUT_BUFFER)?;
    if next_byte!=0 {
        #[allow(const_item_mutation)]
        unsafe { DATA_PORT.write(next_byte) }; // Send command data to COMMAND_REG (i.e. To write to internal RAM)
        poll_bit(STATUS_OUTPUT_BUFFER)?;
    }
    // We know the output buffer status port has been polled, so the data has arrived
    #[allow(const_item_mutation)]
    Ok(unsafe{DATA_PORT.read()})
}

/// Returns true if test passes
pub fn test_controller() -> Result<(), Ps2PollingError> {
    let response = command(0xAA)?;
    if response==0x55 {
        Ok(())
    }
    else if response==0xFC { // Test fails
        //TODO Proper error
        Err(Ps2PollingError::TimeOut)
    } else {
        Err(Ps2PollingError::TimeOut)
    }
}



#[derive(Debug)]
pub enum Ps2PollingError {
    TimeOut
}
