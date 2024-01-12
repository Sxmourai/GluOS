use core::{cell::Cell, fmt::Display, time::Duration};

use spin::Mutex;
use x86_64::instructions::{port::Port, hlt};

use crate::serial_println;

static ELAPSED_TICKS_SINCE_BOOT: Mutex<usize> = Mutex::new(0);
static DATE: Mutex<usize> = Mutex::new(0);




pub struct Date {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub days: u8,
    pub months: u8,
    pub years: u8,
}
impl Date {
    // pub fn as_ticks(&self) -> usize {
    //     // (self.seconds as usize*100)+(self.minutes as usize*100*60)+(self.hours as usize*100*3600)+(self.days as usize*100*3600*24)+(self.months as usize*100*3600*24*30)+(self.years as usize*100*3600*24*365)
    // }
}
impl Display for Date {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&alloc::format!("{}:{}:{}", self.days, self.months, self.years))
    }
}
// TODO Proper CMOS driver ?
const CMOS_ADDRESS: Port<u8> = Port::new(0x70);
const CMOS_DATA: Port<u8> = Port::new(0x71);
#[allow(const_item_mutation)]
fn get_reg(reg: u8) -> u8 {
    unsafe {
        CMOS_ADDRESS.write((1 << 7) | reg);
        CMOS_DATA.read()
    }
}
fn get_update_in_progress_flag() -> bool {
    get_reg(0x0A) & 0x80 != 0
}
// int century_register = 0x00;                                // Set by ACPI table parsing code if possible
pub fn init() {
    // https://wiki.osdev.org/CMOS#Accessing_CMOS_Registers
    // Note: This uses the "read registers until you get the same values twice in a row" technique
    //       to avoid getting dodgy/inconsistent values due to RTC updates
    x86_64::instructions::interrupts::without_interrupts(|| {
        let mut seconds = 0;
        let mut minutes = 0;
        let mut hours = 0;
        let mut days = 0;
        let mut months = 0;
        let mut years = 0;
        while !get_update_in_progress_flag() {
            seconds = get_reg(0x00);
            minutes = get_reg(0x02);
            hours = get_reg(0x04);
            days = get_reg(0x07);
            months = get_reg(0x08);
            years = get_reg(0x09);
            if seconds == get_reg(0x00)
                && minutes == get_reg(0x02)
                && hours == get_reg(0x04)
                && days == get_reg(0x07)
                && months == get_reg(0x08)
                && years == get_reg(0x09)
            {
                break;
            }
        }
        let register_b = get_reg(0x0B);

        // Convert BCD to binary values if necessary
        if (!(register_b & 0x04 != 0)) {
            seconds = (seconds & 0x0F) + ((seconds / 16) * 10);
            minutes = (minutes & 0x0F) + ((minutes / 16) * 10);
            hours = ((hours & 0x0F) + (((hours & 0x70) / 16) * 10)) | (hours & 0x80);
            days = (days & 0x0F) + ((days / 16) * 10);
            months = (months & 0x0F) + ((months / 16) * 10);
            years = (years & 0x0F) + ((years / 16) * 10);
            //TODO if(century_register != 0) {
            //         century = (century & 0x0F) + ((century / 16) * 10);
            // }
        }
        // Convert 12 hour clock to 24 hour clock if necessary
        if (!(register_b & 0x02 != 0) && (hours & 0x80 != 0)) {
            hours = ((hours & 0x7F) + 12) % 24;
        }
        // let current_time = 0;
        let date = Date {
            seconds,
            minutes,
            hours,
            days,
            months,
            years,
        };
        serial_println!("Current time is {}",date);
    });
}

pub fn sleep(time: Duration) {
    loop {

        hlt()
    }
}


pub fn tick() {
    *ELAPSED_TICKS_SINCE_BOOT.lock() += 1;
}
pub fn get_ticks() -> usize {
    ELAPSED_TICKS_SINCE_BOOT.lock().clone()
}
