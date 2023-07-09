//! Multiple APIC Description Table
//! https://wiki.osdev.org/MADT
//The MADT describes all of the interrupt controllers in the system. It can be used to enumerate the processors currently available.
//You can look at the length field in the MADT's header to determine when you have read all the entries.
fn read_rsdp() -> () {
    serial_println!(unsafe { 0x0900 as *const u32 });
}