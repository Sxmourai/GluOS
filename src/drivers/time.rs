use core::cell::Cell;

use spin::Mutex;

use super::{Driver, get_driver};

static ELAPSED_TICKS: Mutex<usize> = Mutex::new(0);

pub fn tick() {
    *ELAPSED_TICKS.lock() += 1;
}
pub fn get_ticks() -> usize {
    ELAPSED_TICKS.lock().clone()
}

pub struct TimeDriver {
    pub ticks: usize,
}
impl TimeDriver {
}

impl Driver for TimeDriver {
    fn new() -> Self where Self: Sized {
        Self {
            ticks: 0,
        }
    }

    fn name(&self) -> &str {
        "Time"
    }

    fn init(&mut self) -> Result<(), super::DriverError> {
        Ok(())
    }

    fn required(&self) -> &str {
        "Interrupts"
    }
}
