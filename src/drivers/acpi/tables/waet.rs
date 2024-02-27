use alloc::format;

use super::ACPISDTHeader;

/// A table from QEMU I think
#[repr(C, packed)]
pub struct WAET {
    header: ACPISDTHeader,
    emu_dev_flags: u32,
}
impl core::fmt::Debug for WAET {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let dev_flags = self.emu_dev_flags;
        return f.debug_struct("WAET")
            .field("header", &self.header)
            .field("emu_dev_flags", &format!("{dev_flags:b}"))
            .finish()
    }
}

/// # Safety
/// Must ensure that bytes is valid WAET !
#[must_use] pub unsafe fn handle_waet(bytes: &[u8]) -> Option<&'static WAET> {
    Some(unsafe { &*bytes.as_ptr().cast::<WAET>() })
}
