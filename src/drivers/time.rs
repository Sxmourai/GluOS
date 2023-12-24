use core::cell::Cell;

use spin::Mutex;


static ELAPSED_TICKS: Mutex<usize> = Mutex::new(0);

pub fn tick() {
    *ELAPSED_TICKS.lock() += 1;
}
pub fn get_ticks() -> usize {
    ELAPSED_TICKS.lock().clone()
}

