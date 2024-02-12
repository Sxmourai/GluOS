mod cmos;
pub mod pit;
pub use cmos::tick;
pub use pit::*;

/// Reads the time from the CMOS battery, it gets UTC time I think
/// We should maybe do a timezone thingy ?
pub fn init() {
    cmos::init()
}
