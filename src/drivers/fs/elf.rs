use alloc::{string::String, vec::Vec};
use bit_field::BitField;
use x86_64::{
    structures::paging::{Page, PageTableFlags, PhysFrame},
    PhysAddr, VirtAddr,
};

use crate::{dbg, mem_handler, mem_map};
const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

///TODO Take a file descriptor, not full content
pub fn execute(content: &[u8]) -> Result<(), ElfError> {
    let bytes = content;
    if bytes[0..4] != ELF_MAGIC {
        // Simple check of magic number
        return Err(ElfError::InvalidEntry);
    }
    let elf = ELF::new(bytes)?;
    let format = elf.start.format().ok_or(ElfError::InvalidEntry)?;
    let mut program_headers = Vec::new();
    for i in 0..elf.end.program_header_entries_count {
        //TODO Should we make a mut offset and then increment it on each iteration ?
        let offset = elf.middle.start_program_header_ptr()
            + u64::from(i * elf.end.program_header_table_entry_size);
        let program_header = ElfProgramHeader::new(&bytes[offset as usize..], format)
            .ok_or(ElfError::InvalidEntry)?;
        program_headers.push(program_header);
    }
    for ph in program_headers {
        let segment_type = ph.segment_type();
        if segment_type.is_none() {
            dbg!();
            continue;
        }
        let segment_type = segment_type.unwrap();
        match segment_type {
            ElfSegmentType::LOAD => {
                if ph
                    .dependant_flags()
                    .ok_or(ElfError::InvalidEntry)?
                    .get_bit(ElfDependantFlags::Executable as usize - 1)
                {
                    //TODO Loop to map if size > 4096 - Map all pages
                    if ph.size_img()>0x1000 {todo!()}
                    let page = Page::containing_address(VirtAddr::new(ph.virt_addr()));
                    let frame = PhysFrame::containing_address(PhysAddr::new(ph.phys_addr()));
                    crate::memory::handler::map_frame(
                        page,
                        frame,
                        PageTableFlags::PRESENT
                            | PageTableFlags::WRITABLE
                            | PageTableFlags::USER_ACCESSIBLE,
                    );
                    // TODO If the p_filesz and p_memsz members differ, this indicates that the segment is padded with zeros. All bytes in memory between the ending offset of the file size, and the segment's virtual memory size are to be cleared with zeros
                    assert_eq!(ph.size_img(), ph.size_mem());
                    let part = &bytes[ph.offset() as usize..(ph.offset() + ph.size_img()) as usize];
                    assert_eq!(ph.size_mem(), part.len() as u64);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            part.as_ptr(),
                            ph.virt_addr() as *mut u8,
                            ph.size_mem() as usize,
                        );
                    };
                }
            }
            ElfSegmentType::DYNAMIC => {
                // todo!()
            }
            _ => {}
        }
    }
    let entry_point_addr = elf.middle.entry();
    crate::println!("Jumping to {:#x}", entry_point_addr);
    //TODO Return from userland, need syscalls & userland, because rn we execute the program in kernel mode
    unsafe {
        core::arch::asm!("
    jmp {:r}
    ", in(reg) entry_point_addr);
    }

    let mut section_headers = Vec::new();
    for i in 0..elf.end.section_header_entries_count {
        let offset = elf.middle.start_section_header_table_ptr() as usize
            + (i as usize) * elf.end.section_header_entry_size as usize;
        let section_header =
            ElfSectionHeader::new(&bytes[offset..], format).ok_or(ElfError::InvalidEntry)?;
        section_headers.push(section_header);
    }
    let names_entry = &section_headers[elf.end.index_section_header_table_entry as usize];

    let names = String::from_utf8_lossy(
        &bytes[names_entry.offset_section_img() as usize
            ..names_entry.offset_section_img() as usize + names_entry.img_size() as usize],
    );
    for header in section_headers {
        let mut name = String::new();
        for char in names[header.name() as usize..].chars() {
            if char == '\0' {
                break;
            }
            name.push(char);
        }
    }
    //     match program_header
    //     .segment_type()
    //     .ok_or(ElfError::InvalidEntry)?
    // {
    //     ElfSegmentType::LOAD => {
    //         //TODO Possible relocation
    //         // mem_map!(
    //         //     frame_addr = program_header.virt_addr(),
    //         //     PageTableFlags::PRESENT
    //         //         | PageTableFlags::USER_ACCESSIBLE
    //         //         | PageTableFlags::WRITABLE
    //         // );
    //         let addr = VirtAddr::new(program_header.virt_addr());
    //         let page = unsafe {
    //             core::slice::from_raw_parts_mut(
    //                 addr.as_mut_ptr::<u8>(),
    //                 program_header.size_mem() as usize,
    //             )
    //         };
    //         let ptr: *const () = addr.as_ptr();
    //         dbg!(1);
    //         let program: extern "C" fn() = unsafe { core::mem::transmute(ptr) };
    //         dbg!(program_header, program);
    //         unsafe{mem_handler!().map(Page::containing_address(addr), PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE)}.unwrap();

    //         // (program)();
    //         dbg!(1);
    //         dbg!(
    //             &bytes[program_header.offset() as usize
    //                 ..program_header.offset() as usize + program_header.size_img() as usize]
    //         );
    //     }
    //     _ => {} //TODO
    // }
    Ok(())
}
#[derive(Debug)]
pub enum ElfSectionHeader<'a> {
    _32(&'a ElfSectionHeader32),
    _64(&'a ElfSectionHeader64),
}
impl ElfSectionHeader<'_> {
    #[must_use]
    pub fn new(bytes: &[u8], format: ElfFormat) -> Option<Self> {
        return Some(match format {
            ElfFormat::_32Bit => {
                Self::_32(unsafe { &*bytes.as_ptr().cast::<ElfSectionHeader32>() })
            }
            ElfFormat::_64Bit => {
                Self::_64(unsafe { &*bytes.as_ptr().cast::<ElfSectionHeader64>() })
            }
        });
    }
    #[must_use]
    pub fn type_header(&self) -> Option<ElfSectionHeaderType> {
        match self {
            ElfSectionHeader::_32(sh) => sh.type_header(),
            ElfSectionHeader::_64(sh) => sh.type_header(),
        }
    }
    #[must_use]
    pub fn offset_section_img(&self) -> u64 {
        match self {
            ElfSectionHeader::_32(sh) => sh.offset_section_img.into(),
            ElfSectionHeader::_64(sh) => sh.offset_section_img,
        }
    }
    #[must_use]
    pub fn name(&self) -> u32 {
        match self {
            ElfSectionHeader::_32(sh) => sh.name,
            ElfSectionHeader::_64(sh) => sh.name,
        }
    }
    #[must_use]
    pub fn img_size(&self) -> u64 {
        match self {
            ElfSectionHeader::_32(sh) => sh.section_size_img.into(),
            ElfSectionHeader::_64(sh) => sh.section_size_img,
        }
    }
    #[must_use]
    pub fn flags(&self) -> Option<ElfSectionHeaderFlags> {
        match self {
            ElfSectionHeader::_32(sh) => sh.flags(),
            ElfSectionHeader::_64(sh) => sh.flags(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ElfSectionHeader32 {
    /// An offset to a string in the .shstrtab section that represents the name of this section
    name: u32,
    /// Identifies the type of this header.
    type_id: u32,
    flags: u32,
    /// Virtual address of the section in memory, for sections that are loaded.
    virt_addr_section_mem: u32,
    /// Offset of the section in the file image.
    offset_section_img: u32,
    /// Size in bytes of the section in the file image. May be 0.
    section_size_img: u32,
    /// Contains the section index of an associated section. This field is used for several purposes, depending on the type of section.
    index_link: u32,
    /// Contains extra information about the section. This field is used for several purposes, depending on the type of section.
    extra_info: u32,
    /// Contains the required alignment of the section. This field must be a power of two.
    align: u32,
    /// Contains the size, in bytes, of each entry, for sections that contain fixed-size entries. Otherwise, this field contains zero.
    entry_size: u32,
}
impl ElfSectionHeader32 {
    #[must_use]
    pub fn type_header(&self) -> Option<ElfSectionHeaderType> {
        ElfSectionHeaderType::from_u32(self.type_id)
    }
    #[must_use]
    pub fn flags(&self) -> Option<ElfSectionHeaderFlags> {
        ElfSectionHeaderFlags::from_u32(self.flags)
    }
}
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct ElfSectionHeader64 {
    /// An offset to a string in the .shstrtab section that represents the name of this section
    name: u32,
    /// Identifies the type of this header.
    type_id: u32,
    flags: u64,
    /// Virtual address of the section in memory, for sections that are loaded.
    virt_addr_section_mem: u64,
    /// Offset of the section in the file image.
    offset_section_img: u64,
    /// Size in bytes of the section in the file image. May be 0.
    section_size_img: u64,
    /// Contains the section index of an associated section. This field is used for several purposes, depending on the type of section.
    index_link: u32,
    /// Contains extra information about the section. This field is used for several purposes, depending on the type of section.
    extra_info: u32,
    /// Contains the required alignment of the section. This field must be a power of two.
    align: u64,
    /// Contains the size, in bytes, of each entry, for sections that contain fixed-size entries. Otherwise, this field contains zero.
    entry_size: u64,
}
impl ElfSectionHeader64 {
    #[must_use]
    pub fn type_header(&self) -> Option<ElfSectionHeaderType> {
        ElfSectionHeaderType::from_u32(self.type_id)
    }
    #[must_use]
    pub fn flags(&self) -> Option<ElfSectionHeaderFlags> {
        ElfSectionHeaderFlags::from_u32(self.flags as u32)
    }
}
#[derive(Debug)]
#[allow(clippy::enum_clike_unportable_variant)]
pub enum ElfSectionHeaderFlags {
    WRITE = 0x1,             //	Writable
    ALLOC = 0x2,             //	Occupies memory during execution
    EXECINSTR = 0x4,         //	Executable
    MERGE = 0x10,            //	Might be merged
    STRINGS = 0x20,          //	Contains null-terminated strings
    InfoLINK = 0x40,         //	'sh_info' contains SHT index
    LinkORDER = 0x80,        //	Preserve order after combining
    OsNONCONFORMING = 0x100, //	Non-standard OS specific handling required
    GROUP = 0x200,           //	Section is member of a group
    TLS = 0x400,             //	Section hold thread-local data
    MASKOS = 0x0FF0_0000,    //	OS-specific
    MASKPROC = 0xF000_0000,  //	Processor-specific
    ORDERED = 0x0400_0000,   //	Special ordering requirement (Solaris)
    EXCLUDE = 0x0800_0000,   //	Section is excluded unless referenced or allocated (Solaris)
}
impl ElfSectionHeaderFlags {
    #[must_use]
    pub fn from_u32(n: u32) -> Option<Self> {
        Some(match n {
            0x1 => Self::WRITE,             //Writable
            0x2 => Self::ALLOC,             //	Occupies memory during execution
            0x4 => Self::EXECINSTR,         //	Executable
            0x10 => Self::MERGE,            //	Might be merged
            0x20 => Self::STRINGS,          //	Contains null-terminated strings
            0x40 => Self::InfoLINK,         //	'sh_info' contains SHT index
            0x80 => Self::LinkORDER,        //	Preserve order after combining
            0x100 => Self::OsNONCONFORMING, //	Non-standard OS specific handling required
            0x200 => Self::GROUP,           //	Section is member of a group
            0x400 => Self::TLS,             //	Section hold thread-local data
            0x0FF0_0000 => Self::MASKOS,    //	OS-specific
            0xF000_0000 => Self::MASKPROC,  //	Processor-specific
            0x0400_0000 => Self::ORDERED,   //	Special ordering requirement (Solaris)
            0x0800_0000 => Self::EXCLUDE, //	Section is excluded unless referenced or allocated (Solaris)
            _ => return None,
        })
    }
}
#[derive(Debug)]
pub enum ElfSectionHeaderType {
    NULL = 0x0,          // Section header table entry unused
    PROGBITS = 0x1,      // Program data
    SYMTAB = 0x2,        // Symbol table
    STRTAB = 0x3,        // String table
    RELA = 0x4,          // Relocation entries with addends
    HASH = 0x5,          // Symbol hash table
    DYNAMIC = 0x6,       // Dynamic linking information
    NOTE = 0x7,          // Notes
    NOBITS = 0x8,        // Program space with no data (bss)
    REL = 0x9,           // Relocation entries, no addends
    SHLIB = 0x0A,        // Reserved
    DYNSYM = 0x0B,       // Dynamic linker symbol table
    InitARRAY = 0x0E,    // Array of constructors
    FiniARRAY = 0x0F,    // Array of destructors
    PreInitArray = 0x10, // Array of pre-constructors
    GROUP = 0x11,        // Section group
    SymtabSHNDX = 0x12,  // Extended section indices
    NUM = 0x13,          // Number of defined types.
    LOOS = 0x6000_0000,  // Start OS-specific.
}
impl ElfSectionHeaderType {
    #[must_use]
    pub fn from_u32(n: u32) -> Option<Self> {
        Some(match n {
            0x0 => Self::NULL,
            0x1 => Self::PROGBITS,
            0x2 => Self::SYMTAB,
            0x3 => Self::STRTAB,
            0x4 => Self::RELA,
            0x5 => Self::HASH,
            0x6 => Self::DYNAMIC,
            0x7 => Self::NOTE,
            0x8 => Self::NOBITS,
            0x9 => Self::REL,
            0x0A => Self::SHLIB,
            0x0B => Self::DYNSYM,
            0x0E => Self::InitARRAY,
            0x0F => Self::FiniARRAY,
            0x10 => Self::PreInitArray,
            0x11 => Self::GROUP,
            0x12 => Self::SymtabSHNDX,
            0x13 => Self::NUM,
            0x6000_0000 => Self::LOOS,
            _ => return None,
        })
    }
}
#[derive(Debug)]
pub enum ElfProgramHeader<'a> {
    _32(&'a ElfProgramHeader32),
    _64(&'a ElfProgramHeader64),
}
impl ElfProgramHeader<'_> {
    #[must_use]
    pub fn virt_addr(&self) -> u64 {
        match self {
            ElfProgramHeader::_32(ph) => u64::from(ph.virt_addr),
            ElfProgramHeader::_64(ph) => ph.virt_addr,
        }
    }
    #[must_use]
    pub fn phys_addr(&self) -> u64 {
        match self {
            ElfProgramHeader::_32(ph) => u64::from(ph.phys_addr),
            ElfProgramHeader::_64(ph) => ph.phys_addr,
        }
    }
    #[must_use]
    pub fn size_mem(&self) -> u64 {
        match self {
            ElfProgramHeader::_32(ph) => u64::from(ph.size_mem),
            ElfProgramHeader::_64(ph) => ph.size_mem,
        }
    }
    #[must_use]
    pub fn size_img(&self) -> u64 {
        match self {
            ElfProgramHeader::_32(ph) => u64::from(ph.size_img),
            ElfProgramHeader::_64(ph) => ph.size_img,
        }
    }
    #[must_use]
    pub fn offset(&self) -> u64 {
        match self {
            ElfProgramHeader::_32(ph) => u64::from(ph.offset),
            ElfProgramHeader::_64(ph) => ph.offset,
        }
    }
    #[must_use]
    pub fn new(bytes: &[u8], format: ElfFormat) -> Option<Self> {
        return Some(match format {
            ElfFormat::_32Bit => {
                Self::_32(unsafe { &*bytes.as_ptr().cast::<ElfProgramHeader32>() })
            }
            ElfFormat::_64Bit => {
                Self::_64(unsafe { &*bytes.as_ptr().cast::<ElfProgramHeader64>() })
            }
        });
    }
    #[must_use]
    pub fn segment_type(&self) -> Option<ElfSegmentType> {
        match self {
            ElfProgramHeader::_32(ph) => ph.segment_type(),
            ElfProgramHeader::_64(ph) => ph.segment_type(),
        }
    }
    #[must_use]
    pub fn dependant_flags(&self) -> Option<u32> {
        match self {
            ElfProgramHeader::_32(_) => None,
            ElfProgramHeader::_64(ph) => Some(ph.dependant_flags()),
        }
    }
}

impl core::fmt::Display for ElfProgramHeader<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ElfProgramHeader::_32(ph) => {
                todo!()
            }
            ElfProgramHeader::_64(ph) => {
                let size = ph.size_img;
                write!(
                    f,
                    "Program header: {:?} {:?} Size: {:?}",
                    ph.segment_type(),
                    ph.dependant_flags(),
                    size
                )
            }
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct ElfProgramHeader32 {
    /// Segment types:
    /// 0 = null - ignore the entry;
    /// 1 = load - clear p_memsz bytes at p_vaddr to 0, then copy p_filesz bytes from p_offset to p_vaddr;
    /// 2 = dynamic - requires dynamic linking;
    /// 3 = interp - contains a file path to an executable to use as an interpreter for the following segment;
    /// 4 = note section.
    /// There are more values, but mostly contain architecture/environment specific information, which is probably not required for the majority of ELF files.
    type_img: u32,
    /// Offset of the segment in the file image
    offset: u32,
    /// Virtual address of the segment in memory
    virt_addr: u32,
    /// On systems where physical address is relevant, reserved for segment's physical address
    phys_addr: u32,
    /// Size in bytes of the segment in the file image. May be 0.
    size_img: u32,
    /// Size in bytes of the segment in memory. May be 0.
    size_mem: u32,
    /// Segment-dependent flags (position for 32-bit structure). See above flags field for flag definitions.
    flags: u32,
    /// 0 and 1 specify no alignment. Otherwise should be a positive, integral power of 2, with p_vaddr equating p_offset modulus p_align.
    align: u32,
}
impl ElfProgramHeader32 {
    #[must_use]
    pub fn segment_type(&self) -> Option<ElfSegmentType> {
        ElfSegmentType::from_type(self.type_img)
    }
}
#[repr(C, packed)]
#[derive(Debug)]
pub struct ElfProgramHeader64 {
    type_img: u32,

    seg_dependant_flags: u32,
    /// Offset of the segment in the file image
    offset: u64,
    /// Virtual address of the segment in memory
    virt_addr: u64,
    /// On systems where physical address is relevant, reserved for segment's physical address
    phys_addr: u64,
    /// Size in bytes of the segment in the file image. May be 0.
    size_img: u64,
    /// Size in bytes of the segment in memory. May be 0.
    size_mem: u64,
    /// 0 and 1 specify no alignment. Otherwise should be a positive, integral power of 2, with p_vaddr equating p_offset modulus p_align.
    align: u64,
}
impl ElfProgramHeader64 {
    #[must_use]
    pub fn segment_type(&self) -> Option<ElfSegmentType> {
        ElfSegmentType::from_type(self.type_img)
    }
    #[must_use]
    pub fn dependant_flags(&self) -> u32 {
        self.seg_dependant_flags
    }
}
#[derive(Debug)]
pub enum ElfDependantFlags {
    Executable = 0x1, // Executable segment.
    Writable = 0x2,   // Writeable segment.
    Readable = 0x4,   // Readable segment
}
#[derive(Debug)]
pub enum ElfSegmentType {
    NULL = 0x0000_0000,    //	Program header table entry unused.
    LOAD = 0x0000_0001,    //	Loadable segment.
    DYNAMIC = 0x0000_0002, //	Dynamic linking information.
    INTERP = 0x0000_0003,  //	Interpreter information.
    NOTE = 0x0000_0004,    //	Auxiliary information.
    SHLIB = 0x0000_0005,   //	Reserved.
    PHDR = 0x0000_0006,    //	Segment containing program header table itself.
    TLS = 0x0000_0007,     //	Thread-Local Storage template.
    LOOS = 0x6000_0000,    //	Reserved inclusive range. Operating system specific.
    HIOS = 0x6FFF_FFFF,    //
    LOPROC = 0x7000_0000,  //	Reserved inclusive range. Processor specific.
    HIPROC = 0x7FFF_FFFF,  //
}
impl ElfSegmentType {
    #[must_use]
    pub fn from_type(_type: u32) -> Option<Self> {
        Some(match _type {
            0x0000_0000 => Self::NULL,    //	Program header table entry unused.
            0x0000_0001 => Self::LOAD,    //	Loadable segment.
            0x0000_0002 => Self::DYNAMIC, //	Dynamic linking information.
            0x0000_0003 => Self::INTERP,  //	Interpreter information.
            0x0000_0004 => Self::NOTE,    //	Auxiliary information.
            0x0000_0005 => Self::SHLIB,   //	Reserved.
            0x0000_0006 => Self::PHDR,    //	Segment containing program header table itself.
            0x0000_0007 => Self::TLS,     //	Thread-Local Storage template.
            0x6FFF_FFFF => Self::HIOS,    //
            0x6000_0000..=0x6FFF_FFFF => Self::LOOS, //	Reserved inclusive range. Operating system specific.
            0x7FFF_FFFF => Self::HIPROC,             //
            0x7000_0000..=0x7FFF_FFFF => Self::LOPROC, //	Reserved inclusive range. Processor specific.
            _ => return None,
        })
    }
}

#[derive(Debug)]
pub enum ElfError {
    InvalidEntry,
}
#[derive(Debug)]
pub struct ELF<'a> {
    start: &'a ELFStart,
    middle: HeaderMiddle,
    end: &'a HeaderEnd,
    size: usize,
}
impl ELF<'_> {
    pub fn new(bytes: &[u8]) -> Result<Self, ElfError> {
        let start = unsafe { &*bytes.as_ptr().cast::<ELFStart>() };
        let (middle, idx) = match start.format().ok_or(ElfError::InvalidEntry)? {
            ElfFormat::_32Bit => (
                HeaderMiddle::_32(unsafe {
                    *bytes[core::mem::size_of::<ELFStart>()..]
                        .as_ptr()
                        .cast::<HeaderMiddle32>()
                }),
                core::mem::size_of::<HeaderMiddle32>(),
            ),
            ElfFormat::_64Bit => (
                HeaderMiddle::_64(unsafe {
                    *bytes[core::mem::size_of::<ELFStart>()..]
                        .as_ptr()
                        .cast::<HeaderMiddle64>()
                }),
                core::mem::size_of::<HeaderMiddle64>(),
            ),
        };
        let end = unsafe {
            &*bytes[core::mem::size_of::<ELFStart>() + idx..]
                .as_ptr()
                .cast::<HeaderEnd>()
        };
        Ok(Self {
            start,
            middle,
            end,
            size: core::mem::size_of::<ELFStart>() + idx + core::mem::size_of::<HeaderEnd>(),
        })
    }
}

#[derive(Debug)]
pub enum HeaderMiddle {
    _32(HeaderMiddle32),
    _64(HeaderMiddle64),
}
impl HeaderMiddle {
    #[must_use]
    pub fn start_section_header_table_ptr(&self) -> u64 {
        match self {
            HeaderMiddle::_32(m) => u64::from(m.start_section_header_table_ptr),
            HeaderMiddle::_64(m) => m.start_section_header_table_ptr,
        }
    }
    #[must_use]
    pub fn entry(&self) -> u64 {
        match self {
            HeaderMiddle::_32(m) => u64::from(m.entry),
            HeaderMiddle::_64(m) => m.entry,
        }
    }
    #[must_use]
    pub fn start_program_header_ptr(&self) -> u64 {
        match self {
            HeaderMiddle::_32(m) => u64::from(m.start_program_header_ptr),
            HeaderMiddle::_64(m) => m.start_program_header_ptr,
        }
    }
}

#[derive(Debug)]
#[repr(C, packed)]
pub struct ELFStart {
    /// 0x7F followed by ELF(45 4c 46) in ASCII; these four bytes constitute the magic number.
    magic: [u8; 4],
    /// This byte is set to either 1 or 2 to signify 32- or 64-bit format, respectively
    bit_format: u8,
    /// This byte is set to either 1 or 2 to signify little or big endianness, respectively. This affects interpretation of multi-byte fields starting with offset 0x10.
    endianness: u8,
    /// Set to 1 for the original and current version of ELF.
    version: u8,
    /// Identifies the target operating system ABI.
    target_abi: u8,
    /// Further specifies the ABI version. Its interpretation depends on the target ABI. Linux kernel (after at least 2.6) has no definition of it,[6] so it is ignored for statically-linked executables. In that case, offset and size of EI_PAD are 8.
    /// glibc 2.12+ in case e_ident[EI_OSABI] == 3 treats this field as ABI version of the dynamic linker:[7] it defines a list of dynamic linker's features,[8] treats e_ident[EI_ABIVERSION] as a feature level requested by the shared object (executable or dynamic library) and refuses to load it if an unknown feature is requested, i.e. e_ident[EI_ABIVERSION] is greater than the largest known feature.[9]
    abi_version: u8,
    _pad: [u8; 7],
    object_file_type: u16,
    /// TODO
    instruction_set_arch: u16,
    /// Set to 1 for the original version of ELF.
    _version: u32,
}
#[derive(Debug)]
#[repr(C, packed)]
pub struct HeaderEnd {
    flags: u32,
    /// Contains the size of this header, normally 64 Bytes for 64-bit and 52 Bytes for 32-bit format.
    header_size: u16,
    /// Contains the size of a program header table entry. As explained below, this will typically be 0x20 (32 bit) or 0x38 (64 bit).
    program_header_table_entry_size: u16,
    /// Contains the number of entries in the program header table.
    program_header_entries_count: u16,
    /// Contains the size of a section header table entry.
    /// As explained below, this will typically be 0x28 (32 bit) or 0x40 (64 bit).
    section_header_entry_size: u16,
    /// Contains the number of entries in the section header table.
    section_header_entries_count: u16,
    /// Contains index of the section header table entry that contains the section names.
    index_section_header_table_entry: u16,
}
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct HeaderMiddle32 {
    /// This is the memory address of the entry point from where the process starts executing.
    /// If the file doesn't have an associated entry point, then this holds zero.
    entry: u32,
    /// Points to the start of the program header table.
    /// It usually follows the file header immediately following this one,
    /// making the offset 0x34 or 0x40 for 32- and 64-bit ELF executables, respectively.
    start_program_header_ptr: u32,
    /// Points to the start of the section header table.
    start_section_header_table_ptr: u32,
}
#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
/// For more info see `HeaderMiddle32`
pub struct HeaderMiddle64 {
    entry: u64,
    start_program_header_ptr: u64,
    start_section_header_table_ptr: u64,
}
impl ELFStart {
    #[must_use]
    pub fn format(&self) -> Option<ElfFormat> {
        match self.bit_format {
            1 => Some(ElfFormat::_32Bit),
            2 => Some(ElfFormat::_64Bit),
            _ => None,
        }
    }
    #[must_use]
    pub fn endianness(&self) -> Option<HeaderEndianness> {
        match self.endianness {
            1 => Some(HeaderEndianness::Little),
            2 => Some(HeaderEndianness::Big),
            _ => None,
        }
    }
    #[must_use]
    pub fn target(&self) -> Option<ElfTarget> {
        match self.target_abi {
            0x00 => Some(ElfTarget::SystemV),
            0x01 => Some(ElfTarget::HpUx),
            0x02 => Some(ElfTarget::NetBSD),
            0x03 => Some(ElfTarget::Linux),
            0x04 => Some(ElfTarget::GnuHurd),
            0x06 => Some(ElfTarget::Solaris),
            0x07 => Some(ElfTarget::AixMonterey),
            0x08 => Some(ElfTarget::IRIX),
            0x09 => Some(ElfTarget::FreeBSD),
            0x0A => Some(ElfTarget::Tru64),
            0x0B => Some(ElfTarget::NovellModesto),
            0x0C => Some(ElfTarget::OpenBSD),
            0x0D => Some(ElfTarget::OpenVMS),
            0x0E => Some(ElfTarget::NonStopKernel),
            0x0F => Some(ElfTarget::AROS),
            0x10 => Some(ElfTarget::FenixOS),
            0x11 => Some(ElfTarget::NuxiCloudAbi),
            0x12 => Some(ElfTarget::StratusTechnologiesOpenVos),
            _ => None,
        }
    }
    #[must_use]
    pub fn object_file_type(&self) -> Option<ElfFileType> {
        Some(match self.object_file_type {
            0x00 => ElfFileType::NONE,     //	Unknown.
            0x01 => ElfFileType::REL,      //	Relocatable file.
            0x02 => ElfFileType::EXEC,     //	Executable file.
            0x03 => ElfFileType::DYN,      //	Shared object.
            0x04 => ElfFileType::CORE,     //	Core file.
            0xFE00 => ElfFileType::LOOS,   //	Reserved inclusive range. Operating system specific.
            0xFEFF => ElfFileType::HIOS,   //	Reserved inclusive range. Operating system specific.
            0xFF00 => ElfFileType::LOPROC, // Reserved inclusive range. Processor specific.
            0xFFFF => ElfFileType::HIPROC, // Reserved inclusive range. Processor specific.
            _ => return None,
        })
    }
}
#[derive(Debug, Copy, Clone)]
pub enum ElfFormat {
    _32Bit,
    _64Bit,
}

pub enum HeaderEndianness {
    Little,
    Big,
}
pub enum ElfTarget {
    SystemV = 0x00,
    HpUx = 0x01,
    NetBSD = 0x02,
    Linux = 0x03,
    GnuHurd = 0x04,
    Solaris = 0x06,
    AixMonterey = 0x07,
    IRIX = 0x08,
    FreeBSD = 0x09,
    Tru64 = 0x0A,
    NovellModesto = 0x0B,
    OpenBSD = 0x0C,
    OpenVMS = 0x0D,
    NonStopKernel = 0x0E,
    AROS = 0x0F,
    FenixOS = 0x10,
    NuxiCloudAbi = 0x11,
    StratusTechnologiesOpenVos = 0x12,
}
pub enum ElfFileType {
    NONE = 0x00,   //	Unknown.
    REL = 0x01,    //	Relocatable file.
    EXEC = 0x02,   //	Executable file.
    DYN = 0x03,    //	Shared object.
    CORE = 0x04,   //	Core file.
    LOOS = 0xFE00, //	Reserved inclusive range. Operating system specific.
    HIOS = 0xFEFF,
    LOPROC = 0xFF00, //	Reserved inclusive range. Processor specific.
    HIPROC = 0xFFFF,
}
