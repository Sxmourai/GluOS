use bit_field::BitField;

use crate::{
    bit_manipulation::{inb, outb},
    dbg,
    interrupts::hardware::{register_interrupt, InterruptIndex},
    ps2::{self, Ps2Controller, DATA_PORT},
    time::mdelay,
};

#[derive(Debug)]
pub struct Mouse {
    pos: (u16, u16),
    /// LEFT, MIDDLE, RIGHT
    click: (bool, bool, bool),
}

pub static mut MOUSE: Mouse = Mouse {
    pos: (u16::MAX / 2, u16::MAX / 2),
    click: (false, false, false),
};

#[derive(Debug)]
pub struct MovementPacket {
    pub misc: u8,
    pub rel_x: u8,
    pub rel_y: u8,
}

pub fn init() {
    Ps2Controller::send_command_next(0xD4, 0xF4);
    Ps2Controller::assert_ack();
    set_sample_rate(10);
    crate::register_interrupt!(
        InterruptIndex::PS2Mouse,
        |_stack_frame| unsafe {
            let misc = Ps2Controller::read_data();
            if misc == 0xFA {
                // It's ACK, so it's not a movement packet
                return;
            }
            let packet = MovementPacket {
                misc,
                rel_x: Ps2Controller::read_data(),
                rel_y: Ps2Controller::read_data(),
            };
            MOUSE.click.0 = packet.misc.get_bit(7);
            MOUSE.click.2 = packet.misc.get_bit(6);
            MOUSE.click.1 = packet.misc.get_bit(5);

            let rel_x = packet.rel_x as i16 - (((packet.misc as i16) << 4) & 0x100);
            let rel_y = packet.rel_y as i16 - (((packet.misc as i16) << 3) & 0x100);

            MOUSE.pos.0 = MOUSE.pos.0.wrapping_add_signed(rel_x);
            MOUSE.pos.1 = MOUSE.pos.1.wrapping_add_signed(rel_y);
        }
    );
}
fn set_sample_rate(rate: u8) {
    // tell the controller to address the mouse & write the mouse command code to the controller's data port
    //TODO Use other method
    Ps2Controller::send_command_next(0xD4, 0xF3);
    Ps2Controller::assert_ack();
    // tell the controller to address the mouse & // write the parameter (sample rate) to the controller's data port
    Ps2Controller::send_command_next(0xD4, rate);
    Ps2Controller::assert_ack();
}
