# GluOS

This is a minimal, modular and lightweight kernel in rust

## Features
- Full rust (with bootloader in assembly+rust, go check [[https://github.com/rust-osdev/bootloader|rust-bootloader]]), with at least unsafe blocks as possible
- Keyboard input (no usb support, ps emulation)
- CPU exceptions and interrupts
- Paging, heap allocation and multitasking
- ATA reading
- FAT32 Reading (dirs & files & recursively)
- Can draw some graphics, but no gui present

## Dev requirements
- Linux system (wsl2 works)
- Nightly rust (should be by default, if not : `rustup override set nightly`)
- qemu (arch: qemu (qemu-full for gui app), debian: qemu-system-x86 (apt))
- rust-src toolchain on nightly rust (`rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu`)
- bootimage (`cargo install bootimage`)
- llvm tools (`rustup component add llvm-tools-preview`)
- run and code ^^ (`cargo run`)
### Caution with WSL !
If you have your compile on wsl but the code is on your local drive (so wsl path is: /mnt/c/.../GluOS) the compile times will be horrible (30s instead of ~2s)
Move it into /home/\[user\] instead

## Additional work
- Our own bootloader (maybe)
- Full Graphical interface
- Write to FAT32
- Other filesystems
- Network, NTP & all
- Optimising & refactoring cuz this code smell bru
- Proper malloc function

## Ressources
- An amazing blog to start out: https://os.phil-opp.com
- Posts on actuality in os dev: https://rust-osdev.com and their github https://github.com/rust-osdev/about
- The "hub" of os deving: https://wiki.osdev.org
- -> List of os's made with Rust https://wiki.osdev.org/Rust
- (a better one): https://github.com/flosse/rust-os-comparison
