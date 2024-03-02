cargo b --release --target x86_64-unknown-none
objcopy -O binary -j .text target/x86_64-unknown-none/release/ap_trampoline ./ap_trampoline.bin