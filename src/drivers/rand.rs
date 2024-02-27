use rand::{rngs::SmallRng, RngCore};
use spin::{Lazy, Mutex};

pub static GENERATOR: Lazy<Mutex<SmallRng>> =
    Lazy::new(|| Mutex::new(rand::SeedableRng::seed_from_u64(get_pseudo())));

/// Don't do anything for now... We could try initialising the Lazy GENERATOR
/// Supposed to init the rdseed
pub fn init() {}

pub fn rand() -> u64 {
    return GENERATOR.lock().next_u64()
}

/// Reads stuff in memory to get some random numbers 🤣
#[must_use] pub fn get_pseudo() -> u64 {
    let mut r = 0;
    for i in 0..10 {
        r += u64::from(unsafe { *((0xF0 + i) as *const u32) });
    }
    r
}
