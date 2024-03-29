#[must_use] pub fn check_msr() -> bool {
    raw_cpuid::CpuId::new()
        .get_feature_info()
        .unwrap()
        .has_msr()
}
/// # Safety
/// The caller must ensure that this read operation has no unsafe side effects
#[must_use] pub unsafe fn read_msr(msr: u32) -> u64 {
    unsafe { x86_64::registers::model_specific::Msr::new(msr).read() }
}
/// # Safety
/// Must ensure that writing to this MSR makes sense and won't break stuff
pub unsafe fn write_msr(msr: u32, value: u64) {
    unsafe { x86_64::registers::model_specific::Msr::new(msr).write(value) }
}
