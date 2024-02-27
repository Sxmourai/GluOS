fn main() {
    println!("cargo:warn=\"Build with: --target x86_64-unknown-none\"");
    println!("cargo:warn=\"Then take only asm: llvm-objcopy target/x86_64-unknown-none/debug/ap_trampoline ./ap_trampoline.bin\"")
}