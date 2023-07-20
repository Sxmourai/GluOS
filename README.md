# GluOS

This is a minimal, modular and lightweight kernel in rust
For instance, based on phil-opp blog: <https://os.phil-opp.com>
https://github.com/thepowersgang/rust_os

## Features

- Full rust (with wrappers in assembly), least unsafe blocks
- Keyboard input (no usb support, ps emulation) // Hardware interrupts
- (some) CPU exceptions
- Paging, heap allocation and multitasking

## Dev requirements

- Linux system (wsl2 works)
- Nightly rust (should be by default, if not : `rustup override set nightly`)
- qemu (arch: qemu (qemu-full for gui app), debian: qemu-system-x86 (apt))
- cmake
- rust-src toolchain on nightly rust (`rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu`)
- bootimage (`cargo install bootimage`)
- llvm tools (`rustup component add llvm-tools-preview`)
- run and code ^^ (`cargo run`)
- One liner : (Without package because it's platform dependent)
´rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu && cargo install bootimage && rustup component add llvm-tools-preview && cargo run´

## Additional work

- Our own bootloader
- Graphical interface (Definitely not today)
- File system

## TODO LIST
- Multiprocessor, get rsdp https://docs.rs/rsdp/latest/src/rsdp/lib.rs.html#47-61, mapping

## Cool ressources
- Phil opp blog
- https://pages.cs.wisc.edu/~remzi/OSTEP/

## To use C functions :
- First use the 'link' macro before using the 'extern' keyword :
#[link(name = "my_c_lib", kind = "static")]
extern "C"
{
    fn my_func(my_args) -> my_return_type; 
}