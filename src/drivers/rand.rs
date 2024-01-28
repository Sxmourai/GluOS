use core::ops::Mul;

use raw_cpuid::{cpuid, native_cpuid::cpuid_count, CpuId};
use spin::{Mutex, Once};

use crate::dbg;

const SEED: Mutex<u128> = Mutex::new(0);
static mut RAND_SUPPORTED: bool = false;

pub fn rand() -> u64 {
    if unsafe{RAND_SUPPORTED} {
        let rand = 0;
        // unsafe{core::arch::asm!("rdseed {rand:e}", rand=in(reg) rand)};
        rand
    } else {
        panic!("Tried random generation but it's not supported")
    }
}
/// Checks if random generator is supported
pub fn init() {
    unsafe{RAND_SUPPORTED = cpuid_count(0, 7).ebx>>18&0x1==1};
}