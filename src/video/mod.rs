use crate::serial_println;

#[derive(Debug)]
#[repr(packed)]
struct VbeInfoBlock {
    vbe_signature: [char; 4],         // == "VESA"
    vbe_version: u16,               // == 0x0300 for VBE 3.0
    oem_str_ptr: [u16;2],         // isa vbeFarPtr
    capabilities: [u8;4],
    video_mode_ptr: [u8;2],         // isa vbeFarPtr
    total_memory: u16,             // as # of 64KB blocks
    reserved: [u8;492],
}

pub fn init_graphics() {
    let info = unsafe { & *(0x2000 as *const VbeInfoBlock) };
    serial_println!("{:?}", info);
}