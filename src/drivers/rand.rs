use rand::{rngs::SmallRng, RngCore};
use spin::{Lazy, Mutex};

pub static GENERATOR: Lazy<Mutex<SmallRng>> =
    Lazy::new(|| return Mutex::new(rand::SeedableRng::seed_from_u64(get_pseudo_rand())));

/// Don't do anything for now... We could try initialising the Lazy GENERATOR
/// Supposed to init the rdseed
pub fn init() {}

pub fn rand() -> u64 {
    return GENERATOR.lock().next_u64()
}

/// Reads stuff in memory to get some random numbers 🤣
pub fn get_pseudo_rand() -> u64 {
    let mut r = 0;
    for i in 0..10 {
        r += unsafe { *((0xF0 + i) as *const u32) } as u64;
    }
    return r
}
