use x86_64::structures::port::{PortRead, PortWrite};

use crate::dbg;

const BASE_FREQUENCY: u64 = 1_193_182;
#[derive(Debug)]
pub enum TimerError {
    OutOfRange,
    NotActive,
}
// Inline it because we call it a million times to wait for 1 second
#[inline(always)]
pub fn udelay(micros: u16) -> Result<(), TimerError> {
    set(micros)?;
    wait_for_timeout()
}
pub fn mdelay(millis: u16) -> Result<(), TimerError> {
    let micros = millis as u64*1_000;
    let times = micros/MAX_COUNTER_VALUE_INPUT as u64;
    let rest = (micros%MAX_COUNTER_VALUE_INPUT as u64) as u16;
    dbg!(times, rest);
    for i in 0..times {
        udelay(MAX_COUNTER_VALUE_INPUT)?
    }
    udelay(rest);
    Ok(())
}
pub fn sdelay(seconds: u16) -> Result<(), TimerError> {
    let micros = seconds as u64*1_000_000;
    let times = micros/MAX_COUNTER_VALUE_INPUT as u64;
    let rest = (micros%MAX_COUNTER_VALUE_INPUT as u64) as u16;
    dbg!(times, rest);
    for i in 0..times {
        udelay(MAX_COUNTER_VALUE_INPUT)?
    }
    udelay(rest);
    Ok(())
}

pub fn wait_for_timeout() -> Result<(), TimerError> {
    loop {
        let c = Control(unsafe { PortRead::read_from_port(0x61 as u16) });
        if !c.enable_timer_counter2() {
            return Err(TimerError::NotActive);
        } else if c.status_timer_counter2() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
}
pub const MAX_COUNTER_VALUE: u16 = 0xffff;
pub const MAX_COUNTER_VALUE_INPUT: u16 = ((MAX_COUNTER_VALUE as u64*1_000_000u64)/BASE_FREQUENCY) as u16; //We could use .div_floor to be more explicit, but it's unstable ~= 54924.5630591142
pub fn set(micros: u16) -> Result<(), TimerError> {
    let counter = (BASE_FREQUENCY*micros as u64) / 1_000_000;
    if counter > MAX_COUNTER_VALUE as u64 {
        return Err(TimerError::OutOfRange);
    }
    let mut c = Control(unsafe { PortRead::read_from_port(0x61 as u16) });
    c.set_enable_speaker_data(false);
    c.set_enable_timer_counter2(true);
    unsafe { PortWrite::write_to_port(0x61, c.0) }

    let mut m = Mode(0);
    m.set_bcd_format(false);
    m.set_access_mode(AccessMode::LowAndHighByte as u8);
    m.set_operating_mode(OperatingMode::InterruptOnTerminalCount as u8);
    m.set_channel(2);
    unsafe { PortWrite::write_to_port(0x43, m.0) }

    set_data(2, (counter & 0xff) as u8);
    set_data(2, ((counter >> 8) & 0xff) as u8);
    Ok(())
}

fn set_data(channel: u8, value: u8) {
    unsafe { PortWrite::write_to_port(0x40 + channel as u16, value) }
}

fn get_data(channel: u8) -> u8 {
    unsafe { PortRead::read_from_port(0x40 + channel as u16) }
}

bitfield::bitfield! {
    #[derive(Clone, Copy)]
    struct Mode(u8);
    impl Debug;
    _, set_bcd_format          : 0;
    _, set_operating_mode   : 3, 1;
    _, set_access_mode      : 5, 4;
    _, set_channel          : 7, 6;
}
bitfield::bitfield! {
    #[derive(Clone, Copy)]
    struct Control(u8);
    impl Debug;
    enable_timer_counter2, set_enable_timer_counter2 : 0;
    enable_speaker_data  , set_enable_speaker_data   : 1;
    enable_pci_serr      , set_enable_pci_serr       : 2;
    enable_nmi_iochk     , set_enable_nmi_iochk      : 3;
    refresh_cycle_toggle   , _ : 4;
    status_timer_counter2  , _ : 5;
    status_iochk_nmi_source, _ : 6;
    status_serr_nmi_source , _ : 7;
}

#[repr(u8)]
enum AccessMode {
    LatchCountValue = 0,
    LowByteOnly = 1,
    HighByteOnly = 2,
    LowAndHighByte = 3,
}

#[repr(u8)]
enum OperatingMode {
    InterruptOnTerminalCount = 0,
    ProgrammableOneShot = 1,
    RateGenerator = 2,
    SquareWaveGenerator = 3,
    SoftwareTriggeredStrobe = 4,
    HardwareTriggeredStrobe = 5,
}