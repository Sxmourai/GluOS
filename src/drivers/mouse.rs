use crate::bit_manipulation::{inb, outb};

pub fn init() {
    outb(0xF3, 0x60); // write the mouse command code to the controller's data port
    while inb(0x64) & 1 == 1 {} // wait until we can read
    let ack = inb(0x60); // read back acknowledge. This should be 0xFA
    outb(0xD4, 0x64); // tell the controller to address the mouse
    outb(100, 0x60); // write the parameter to the controller's data port
    while inb(0x64) & 1 == 1 {} // wait until we can read
    let ack = inb(0x60); // read back acknowledge. This should be 0xFA
}
