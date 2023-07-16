pub mod idt;
pub mod exceptions;
pub mod hardware;

pub use idt::IDT;
pub use hardware::PICS;
pub use hardware::{add_input,get_input_msg,remove_input};