# GluOS

This is a minimal, modular and lightweight kernel in rust

## Features
- Full rust (with bootloader in assembly+rust, go check https://github.com/rust-osdev/bootloader), with at least unsafe blocks as possible
- Keyboard input (no usb support, ps emulation)
- CPU exceptions and interrupts
- Paging, heap allocation and multitasking
- ATA reading
- Fat32, ext2, NTFS (only read)
- Can draw some graphics, but no gui present
- Timer delay (no interrupts for now)

### Working on features
- NVMe (see src/drivers/disk/nvme.rs)
- Ethernet (see src/drivers/network/e1000.rs)
- ELF loading (see src/drivers/elf.rs & the associated command in src/user/shell.rs -> Execute)

## Dev requirements
- Linux system (wsl2 works)
- Nightly rust (should be by default, if not : `rustup override set nightly`)
- qemu (arch: qemu (qemu-full for gui app), debian: qemu-system-x86 (apt))
- rust-src toolchain on nightly rust (`rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu`)
- llvm tools (`rustup component add llvm-tools-preview`)
- bootimage (`cargo install bootimage`)
- LLD
- run and code ^^ (`cargo run`)
- Btw `cargo build` doesn't work but it's normal !

### Caution with WSL !
If you have your compile on wsl but the code is on your local drive (so wsl path is: /mnt/c/.../GluOS) the compile times will be horrible (30s instead of ~2s)
Move it into /home/\[user\] instead

## Additional work
- Our own bootloader (maybe)
- Full Graphical interface
- Network, NTP & all
- More drivers:
- NVMe
- Ethernet
- USB
- Optimising & refactoring

## Ressources
- An amazing blog to start out: https://os.phil-opp.com
- Posts on actuality in os dev: https://rust-osdev.com and their github https://github.com/rust-osdev/about
- The main wiki of os deving: https://wiki.osdev.org (has been my goto for nearly everything)

### Some other os's in Rust
- https://github.com/nuta/kerla/tree/main/kernel
- https://github.com/llenotre/maestro
- https://github.com/intermezzOS/kernel
- https://github.com/hermit-os/hermit-rs
- And for more:
- https://wiki.osdev.org/Rust
- https://github.com/flosse/rust-os-comparison

## Stuff to check
- `cargo audit`
- `cargo tree --duplicate`
- Kernel size, compile times, and speed (the most significant way of seeing speed is the time it takes to init descriptor tables for now)
- Speed: https://corrode.dev/blog/tips-for-faster-rust-compile-times/
- Size: https://github.com/johnthagen/min-sized-rust
- Fast: 
