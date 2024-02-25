#[repr(C, packed)]
pub struct HPET {
    header: super::ACPISDTHeader,
    hardware_rev_id: u8,
    comparator_count: u8,
    counter_size: u8,
    reserved: u8,
    legacy_replacement: u8,
    pci_vendor_id: u16,
    address: super::GenericAddressStructure,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8,
}
/// Needs implement because packed struct
impl core::fmt::Debug for HPET {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let min_tick = self.minimum_tick;
        let vendor_id = self.pci_vendor_id;
        f.debug_struct("HPET")
            .field("header", &self.header)
            .field("hardware_rev_id", &self.hardware_rev_id)
            .field("comparator_count", &self.comparator_count)
            .field("counter_size", &self.counter_size)
            .field("reserved", &self.reserved)
            .field("legacy_replacement", &self.legacy_replacement)
            .field("pci_vendor_id", &vendor_id)
            .field("address", &self.address)
            .field("hpet_number", &self.hpet_number)
            .field("minimum_tick", &min_tick)
            .field("page_protection", &self.page_protection)
            .finish()
    }
}

/// # Safety
/// Must ensure bytes is proper HPET
pub unsafe fn handle_hpet(bytes: &[u8]) -> Option<&'static HPET> {
    Some(unsafe { &*(bytes.as_ptr() as *const HPET) })
}
