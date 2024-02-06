use crate::{bit_manipulation::{inb, outb}, dbg};

pub fn init() {
    outb(0xD4, 0x64);                    // tell the controller to address the mouse
    outb(0xFF, 0x00);
    dbg!(inb(0x64));
    while inb(0x64) & 1 == 1 {} // wait until we can read
    assert_ack();
    outb(0xF3, 0x60); // write the mouse command code to the controller's data port
    while inb(0x64) & 1 == 1 {} // wait until we can read
    assert_ack();
    outb(0xD4, 0x64); // tell the controller to address the mouse
    outb(100, 0x60); // write the parameter to the controller's data port
    while inb(0x64) & 1 == 1 {} // wait until we can read
    assert_ack();
    
}

fn assert_ack() -> bool {
    let ack = inb(0x60); // read back acknowledge. This should be 0xFA
    if ack != 0xFA {
        log::error!("Failed initialising mouse driver !")
    }
    ack != 0xFA
}