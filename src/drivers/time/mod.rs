mod cmos;
pub mod pit;
pub use cmos::tick;
pub use pit::*;

/// Reads the time from the CMOS battery, it gets UTC time I think
/// We should maybe do a timezone thingy ?
pub fn init() {
    cmos::init();
    pit::init();
}

pub fn try_udelay(micros: u16) -> Result<(), TimerError> {
    set(micros)?;
    return wait_for_timeout()
}
pub fn try_mdelay(millis: u32) -> Result<(), TimerError> {
    let id = pit::register_wait().ok_or(TimerError::NoTicksAvailable)?;
    while pit::get_ticks(id).unwrap()<=millis {
        x86_64::instructions::hlt();
    }
    log::debug!("Finished waiting");
    return Ok(())
}
pub fn try_sdelay(seconds: u32) -> Result<(), TimerError> {
    if seconds>=u32::MAX/1000 {return Err(TimerError::OutOfRange)}
    return try_mdelay(seconds*1000)
}
pub fn udelay(micros: u16) {
    try_udelay(micros).unwrap()
}
pub fn mdelay(millis: u32) {
    try_mdelay(millis).unwrap()
}
//TODO Make real async
pub fn sdelay(seconds: u32) {
    try_sdelay(seconds).unwrap()
}