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

### Caution with WSL !
If you have your compile on wsl but the code is on your local drive (so wsl path is: /mnt/c/.../GluOS) the compile times will be horrible (30s instead of ~2s)
Move it into /home/\[user\] instead

## Additional work
- Our own bootloader (maybe)
- Full Graphical interface
- Network, NTP & all
- Optimising & refactoring

## Ressources
- An amazing blog to start out: https://os.phil-opp.com
- Posts on actuality in os dev: https://rust-osdev.com and their github https://github.com/rust-osdev/about
- The "hub" of os deving: https://wiki.osdev.org
- -> List of os's made with Rust https://wiki.osdev.org/Rust
- (a better one): https://github.com/flosse/rust-os-comparison

## Stuff to check
- `cargo audit`
- `cargo tree --duplicate`
- Kernel size, compile times, and speed (the most significant way of seeing speed is the time it takes to init descriptor tables for now)
- Speed: https://corrode.dev/blog/tips-for-faster-rust-compile-times/
- Size: https://github.com/johnthagen/min-sized-rust
- Fast: 
