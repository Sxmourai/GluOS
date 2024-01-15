
pub fn check_msr() -> bool {
    raw_cpuid::CpuId::new().get_feature_info().unwrap().has_msr()
}
pub unsafe fn get_msr(msr: u32) -> u64 {
    unsafe{x86_64::registers::model_specific::Msr::new(msr).read()}
}
pub unsafe fn set_msr(msr: u32, value: u64) {
    unsafe{x86_64::registers::model_specific::Msr::new(msr).write(value)}
}