use super::{DiskLoc, SELECTED_DISK};

pub fn primary_bus_irq() {
    common();
}
pub fn secondary_bus_irq() {
    common();
}

pub fn common() {
    let raw_selected_disk = SELECTED_DISK.load(core::sync::atomic::Ordering::Acquire);
    let selected_disk = DiskLoc::from_idx(raw_selected_disk).unwrap();
    // crate::dbg!(selected_disk);
}
