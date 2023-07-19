#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate kernel;
extern crate lazy_static;
extern crate x86_64;

use core::panic::PanicInfo;
use kernel::{exit_qemu, QemuExitCode, serial_println, serial_print};
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

extern "C"
{
    fn c_add(a: i32, b: i32) -> i32;
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_print!("Testing C functions: {}", unsafe{c_add(1, 2)}); 
    serial_print!("stack_overflow::stack_overflow...\t");

    kernel::gdt::init();
    init_test_idt();

    // trigger a stack overflow
    stack_overflow();

    panic!("Execution continued after stack overflow");
}

#[allow(unconditional_recursion)]
fn stack_overflow() {
    stack_overflow(); // for each recursion, the return address is pushed
    volatile::Volatile::new(0).read(); // prevent tail recursion optimizations
}

lazy_static! {
    static ref TEST_IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(test_double_fault_handler)
                .set_stack_index(kernel::gdt::DOUBLE_FAULT_IST_INDEX);
        }

        idt
    };
}extern "x86-interrupt" fn test_double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    serial_println!("[ok]");
    exit_qemu(QemuExitCode::Success);

    kernel::end()
}

pub fn init_test_idt() {
    TEST_IDT.load();
}



#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    kernel::test_panic_handler(info)
}