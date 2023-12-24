// use core::ffi::c_uchar;

// use crate::writer::{outb, inb};


// const VGA_AC_INDEX		:u16=0x3C0;
// const VGA_AC_WRITE		:u16=0x3C0;
// const VGA_AC_READ		:u16=0x3C1;
// const VGA_MISC_WRITE	:u16=	0x3C2;
// const VGA_SEQ_INDEX		:u16=0x3C4;
// const VGA_SEQ_DATA		:u16=0x3C5;
// const VGA_DAC_READ_INDEX:u16=	0x3C7;
// const VGA_DAC_WRITE_INDEX:u16=	0x3C8;
// const VGA_DAC_DATA		:u16=0x3C9;
// const VGA_MISC_READ		:u16=0x3CC;
// const VGA_GC_INDEX 		:u16=0x3CE;
// const VGA_GC_DATA 		:u16=0x3CF;
// /*			COLOR emulation		MONO emulation */
// const VGA_CRTC_INDEX:u16=		0x3D4		/* 0x3B4 */;
// const VGA_CRTC_DATA:u16=		0x3D5		/* 0x3B5 */;
// const VGA_INSTAT_READ:u16=		0x3DA;

// const VGA_NUM_SEQ_REGS:u8=	5;
// const VGA_NUM_CRTC_REGS:u8=	25;
// const VGA_NUM_GC_REGS:u8=		9;
// const VGA_NUM_AC_REGS:u8=		21;
// const VGA_NUM_REGS:u8=		(1 + VGA_NUM_SEQ_REGS + VGA_NUM_CRTC_REGS + VGA_NUM_GC_REGS + VGA_NUM_AC_REGS);

// unsafe fn write_regs(mut regs: &[c_uchar]) {
//     let mut regs = &mut regs[..];
//     let mut j = 0;
    
//     /* write MISCELLANEOUS reg */
//     outb(VGA_MISC_WRITE, regs[j]);
//     j+=1;
//     /* write SEQUENCER regs */
//     for i in 0..VGA_NUM_SEQ_REGS {
//         outb(VGA_SEQ_INDEX, i);
//         outb(VGA_SEQ_DATA, regs[j]);
//         j+=1;
//     }
//     /* unlock CRTC registers */
//     outb(VGA_CRTC_INDEX, 0x03);
//     outb(VGA_CRTC_DATA, inb(VGA_CRTC_DATA) | 0x80);
//     outb(VGA_CRTC_INDEX, 0x11);
//     outb(VGA_CRTC_DATA, inb(VGA_CRTC_DATA) & !0x80);
//     /* make sure they remain unlocked */
//     regs[0x03] |= 0x80;
//     regs[0x11] &= !0x80;
//     /* write CRTC regs */
//     for i in 0..VGA_NUM_CRTC_REGS {
//         outb(VGA_CRTC_INDEX, i);
//         outb(VGA_CRTC_DATA, regs[j]);
//         j+=1;
//     }
//     /* write GRAPHICS CONTROLLER regs */
//     for i in 0..VGA_NUM_GC_REGS {
//         outb(VGA_GC_INDEX, i);
//         outb(VGA_GC_DATA, regs[j]);
//         j+=1;
//     }
//     /* write ATTRIBUTE CONTROLLER regs */
//     for i in 0..VGA_NUM_AC_REGS {
//         let _ = inb(VGA_INSTAT_READ);
//         outb(VGA_AC_INDEX, i);
//         outb(VGA_AC_WRITE, regs[j]);
//         j+=1;
//     }
//     /* lock 16-color palette and unblank display */
//     let _ = inb(VGA_INSTAT_READ);
//     outb(VGA_AC_INDEX, 0x20);
// }