# GluOS

This is a minimal, modular and lightweight kernel in rust
For instance, based on phil-opp blog: <https://os.phil-opp.com>

## Dev requirements

- Linux system (wsl2 works)
- Nightly rust (should be by default, if not : `rustup override set nightly`)
- qemu (arch: pacman -S qemu (qemu-full for app) )
- rust-src toolchain on nightly rust (`rustup component add rust-src --toolchain nightly-x86_64-unknown-linux-gnu`)
- bootimage (`cargo install bootimage`)
- llvm tools (`rustup component add llvm-tools-preview`)
