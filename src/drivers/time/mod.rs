mod cmos;
pub mod pit;
pub use cmos::tick;
pub use pit::*;

pub fn init() {
    cmos::init()
}
