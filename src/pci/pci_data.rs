use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

use crate::serial_println;

pub fn print_all_pci_classes_and_subclasses() {
    for class in pci_ids::Classes::iter() {
        serial_println!("Class: {} (id: {})", class.name(), class.id());
        let subclasses = class.subclasses().map(|sbc| format!("- {} (id: {})",sbc.name(), sbc.id())).collect::<Vec<String>>().join("\n");
        serial_println!("Subclasses: \n{}\n----------------------------------", subclasses);
    }
}

pub fn print_all_pci_devices() {
    for device in crate::pci::pci_device_iter() {
        let d = pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id).expect(&alloc::format!("Not found, {:?}", device));
        let subs: Vec<&'static pci_ids::SubSystem> = d.subsystems().collect();
        
        let vendor = d.vendor().name();
        let class = device.class;
        serial_println!("Device {} - Vendor {:?} - Class {} sub:{} - Subsystems {:?} - ON BUS: {}",d.name(), vendor, class, device.subclass, subs, device.bus());
    }
}