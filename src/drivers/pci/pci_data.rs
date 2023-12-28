use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::serial_println;

pub fn print_all_pci_classes_and_subclasses() {
    for class in pci_ids::Classes::iter() {
        serial_println!("Class: {} (id: {})", class.name(), class.id());
        let subclasses = class
            .subclasses()
            .map(|sbc| format!("- {} (id: {})", sbc.name(), sbc.id()))
            .collect::<Vec<String>>()
            .join("\n");
        serial_println!(
            "Subclasses: \n{}\n----------------------------------",
            subclasses
        );
    }
}

// Go on the discord, into #ressources to have the formated output
pub fn print_all_pci_devices() {
    for device in crate::pci::pci_device_iter() {
        let mut name;
        let mut subs;
        let mut vendor;
        let mut class = "Not found";
        let mut subclass = "Not found";
        if device.vendor_id == 0x1234 && device.device_id == 0x1111 { //TODO This is a workaround, because idk why it doesn't find this device in pci.ids but it's on the website
            name = "QEMU Virtual Video Controller";
            vendor = "QEMU"; // Any Some other hypervisors use this device
            subs = Vec::new();
        } else {
            let d = pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id).expect(&alloc::format!("Not found, {:?}", device));
            name = d.name();
            subs = d.subsystems().collect();
            vendor = d.vendor().name(); 
            
            for iter_class in pci_ids::Classes::iter() {
                if iter_class.id() == device.class {
                    for iter_subclass in iter_class.subclasses() {
                        if iter_subclass.id() == device.subclass { //TODO Don't be afraid of nesting
                            class = iter_class.name();
                            subclass = iter_subclass.name();
                        }
                    }
                }
            }
        }
        
        subs[0].name();
        serial_println!(
            "BUS: {}\t- {}\t-\tVendor {:?}\nClass: {}\t-\tSubclass: {}\nSubsystems {:?}\n\n",
            device.bus(),
            name,
            vendor,
            class,
            subclass,
            subs,
        );
    }
}

// Go on the discord, into #ressources to have the formated output
pub fn print_all_pci_devices_big() {
    for device in crate::pci::pci_device_iter() {
        let mut name;
        let mut subs;
        let mut vendor;
        let mut class = "Not found";
        let mut subclass = "Not found";
        if device.vendor_id == 0x1234 && device.device_id == 0x1111 { //TODO This is a workaround, because idk why it doesn't find this device in pci.ids but it's on the website
            name = "QEMU Virtual Video Controller";
            vendor = "QEMU"; // Any Some other hypervisors use this device
            subs = Vec::new();
        } else {
            let d = pci_ids::Device::from_vid_pid(device.vendor_id, device.device_id).expect(&alloc::format!("Not found, {:?}", device));
            name = d.name();
            subs = d.subsystems().collect();
            vendor = d.vendor().name(); 
            
            for iter_class in pci_ids::Classes::iter() {
                if iter_class.id() == device.class {
                    for iter_subclass in iter_class.subclasses() {
                        if iter_subclass.id() == device.subclass { //TODO Don't be afraid of nesting
                            class = iter_class.name();
                            subclass = iter_subclass.name();
                        }
                    }
                }
            }
        }
        
        subs[0].name();
        serial_println!("{:?}", device);
    }
}
