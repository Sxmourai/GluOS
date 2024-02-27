use core::{cell::OnceCell, sync::atomic::AtomicU32};

use alloc::vec::Vec;
use spin::RwLock;
use x86_64::{
    instructions::interrupts::without_interrupts,
    structures::port::{PortRead, PortWrite},
};

use crate::dbg;

pub fn irq() {
    if let Some(controller) = unsafe { PIT_CONTROLLER.get_mut() } {
        for tick in controller.ticks.iter_mut() {
            tick.fetch_add(1, core::sync::atomic::Ordering::Release);
        }
        // We can be sure that we are the only ones modifying this value, because it's only changed when a timer interrupt occurs
        controller.elapsed_ticks+=1;
        #[cfg(debug_assertions)]
        if controller.elapsed_ticks / SELECTED_HZ as u128 == 60*60 {
            log::info!("Kernel has been running for 1 hour, wow");
        }
    }
}

/// Creates a new entry in ticks and returns it's id
pub fn register_wait() -> Option<usize> {
    unsafe{PIT_CONTROLLER.get_mut().map(|c| {
        c.ticks.push(AtomicU32::new(0));
        return c.ticks.len()-1
    })}
}
/// Gets the ticks from the id, if it exists
pub fn get_ticks(id: usize) -> Option<u32> {
    unsafe{PIT_CONTROLLER.get_mut().and_then(|c| {
        return Some(c.ticks.get(id)?.load(core::sync::atomic::Ordering::Acquire))
    })}
}

// TODO Use OnceCell, but doesn't have Sync
pub static mut PIT_CONTROLLER: OnceCell<PIT> = OnceCell::new();
pub const MIN_FREQUENCY: u32 = 18.222 as u32;
pub const PIT_FREQUENCY: u32 = 1_193_181;
// Because 18.222 hz = 1 second, we want 18222 to have interrupts every ms
pub const SELECTED_HZ: u32 = 1000;
pub fn init() {
    if SELECTED_HZ <= MIN_FREQUENCY {
        return;
    }
    let divisor = PIT_FREQUENCY / SELECTED_HZ;
    let mut mode = Mode(0);
    mode.set_access_mode(AccessMode::AccessModeLoHiByte as u8);
    mode.set_operating_mode(OperatingMode::SquareWaveGenerator as u8);
    mode.set_bcd_format(false);
    mode.set_channel(Channel::Channel0 as u8);
    mode.write();
    PIT::write_reg(Regs::Channel0, divisor as u8);
    PIT::write_reg(Regs::Channel0, ((divisor & 0xFF00) >> 8) as u8);

    let mut pit = PIT {
        ticks: Vec::new(),
        elapsed_ticks: 0,
    };
    unsafe { PIT_CONTROLLER.set(pit); }
}

#[derive(Debug)]
#[repr(u8)]
pub enum Channel {
    Channel0,
    Channel1,
    Channel2,
    /// 8254 only
    ReadBackCommand,
}
#[derive(Debug)]
pub enum AccessMode {
    LatchCountValueCommand,
    AccessModeLoByteOnly,
    AccessModeHiByteOnly,
    AccessModeLoHiByte,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Regs {
    Channel0 = 0x40,
    Channel1 = 0x41,
    Channel2 = 0x42,
    ModeCommand = 0x43,
    Control = 0x61,
}
impl Regs {
    pub fn from_channel(channel: Channel) -> Self {
        match channel {
            Channel::Channel0 => return Regs::Channel0,
            Channel::Channel1 => return Regs::Channel1,
            Channel::Channel2 => return Regs::Channel2,
            Channel::ReadBackCommand => todo!(),
        }
    }
}

pub struct PIT {
    /// Stores the different amount of ticks that has elapsed since the registering of a sleep
    /// TODO make ^^ clearer
    ticks: Vec<core::sync::atomic::AtomicU32>,
    /// Stores the amount of ticks that have elapsed since boot
    elapsed_ticks: u128,
}
impl PIT {
    pub fn write_reg(reg: Regs, value: u8) {
        unsafe { PortWrite::write_to_port(reg as u16, value) }
    }
    pub fn read_reg(reg: Regs) -> u8 {
        unsafe { return PortRead::read_from_port(reg as u16) }
    }
    pub fn read_pit_count(channel: Channel) -> u16 {
        without_interrupts(|| {
            let channel_reg = Regs::from_channel(channel);
            Self::write_reg(Regs::ModeCommand, 0);
            return ((Self::read_reg(channel_reg) as u16) | (Self::read_reg(channel_reg) as u16) << 8)
        })
    }
    //TODO Handle LoByte Only and Hi
    pub fn write_pit_count(&self, channel: Channel, value: u16) {
        without_interrupts(|| {
            let channel_reg = Regs::from_channel(channel);
            Self::write_reg(channel_reg, value as u8);
            Self::write_reg(channel_reg, ((value & 0xFF00) >> 8) as u8);
        })
    }
}

const BASE_FREQUENCY: u64 = 1_193_182;
#[derive(Debug)]
pub enum TimerError {
    OutOfRange,
    NotActive,
    NoTicksAvailable,
}

pub fn wait_for_timeout() -> Result<(), TimerError> {
    loop {
        let c = Control::read();
        if !c.enable_timer_counter2() {
            log::error!("Timer not active");
            return Err(TimerError::NotActive);
        } else if c.status_timer_counter2() {
            return Ok(());
        }
        core::hint::spin_loop();
    }
}
pub const MAX_COUNTER_VALUE: u16 = u16::MAX;
pub const MAX_COUNTER_VALUE_INPUT: u16 =
    ((MAX_COUNTER_VALUE as u64 * 1_000_000_u64) / BASE_FREQUENCY) as u16; // ~= 54924.5630591142
pub fn set(micros: u16) -> Result<(), TimerError> {
    let counter = (BASE_FREQUENCY * micros as u64) / 1_000_000;
    if counter > MAX_COUNTER_VALUE as u64 {
        return Err(TimerError::OutOfRange);
    }
    let mut control = Control::read();
    control.set_enable_speaker_data(false);
    control.set_enable_timer_counter2(true);
    control.write();

    let mut m = Mode(0);
    m.set_bcd_format(false);
    m.set_access_mode(AccessMode::AccessModeLoHiByte as u8);
    m.set_operating_mode(OperatingMode::InterruptOnTerminalCount as u8);
    m.set_channel(2);
    m.write();

    PIT::write_reg(Regs::Channel2, (counter & 0xff) as u8);
    PIT::write_reg(Regs::Channel2, ((counter >> 8) & 0xff) as u8);
    return Ok(())
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
impl Mode {
    pub fn read() -> Self {
        return Self(PIT::read_reg(Regs::ModeCommand))
    }
    pub fn write(&self) {
        PIT::write_reg(Regs::ModeCommand, self.0)
    }
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
impl Control {
    pub fn read() -> Self {
        return Self(PIT::read_reg(Regs::Control))
    }
    pub fn write(&self) {
        PIT::write_reg(Regs::Control, self.0)
    }
}

#[repr(u8)]
pub enum OperatingMode {
    InterruptOnTerminalCount = 0,
    ProgrammableOneShot = 1,
    RateGenerator = 2,
    SquareWaveGenerator = 3,
    SoftwareTriggeredStrobe = 4,
    HardwareTriggeredStrobe = 5,
}
