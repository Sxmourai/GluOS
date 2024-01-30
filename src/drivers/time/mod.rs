mod cmos;
pub mod pit;
pub use pit::*;
pub use cmos::tick;

pub fn init() {
    cmos::init()
}