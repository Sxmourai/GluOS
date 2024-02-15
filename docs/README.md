# Introduction
This is the (very incomplete) docs for GluOS

# Architecture
In "src" folder the os is separated into "drivers" which are modules that directly interact with hardware and "user" contains modules that interact with drivers but doesn't interact directly with hardware.
All functions that needs to be inited are inited in "src/drivers/mod.rs", but the main function is in "src/boot.rs".

The boot order goes this way: (this might change A LOT)
- bootloader (phill opp's)
- main function in "src/main.rs"
- boot() in "src/boot.rs"
- Initialises drivers in src/drivers/mod.rs
- Starts the executor and makes it run (for async & all)

### Drivers
All paths are relative to "src/drivers"
- [ACPI](acpi.md): Usefull for PS/2 & other stuff
- [Disk](disk.md): ATA reading (working on NVMe)
- [File systems](fs.md): FAT32 & Ext2 & NTFS
- [Graphics](graphics.md): Vga text buffer...
- [Interrupts](interrupts.md): Hot load IDT
- [Memory](memory.md): Heap allocation, frame mapping & global allocator
- [Network](net.md): Nothing for now, but will have WIFI & Ethernet
- [PCI](pci.md): Pci devices scanning & parsing
- [Multiprocessing](smp.md): SMP Multiprocessing not supported (but should use all cores)
- [Executor & Tasks](task.md): An executor to use async/await in the os, but not used much
- [Time](time.md): PIT for delays... And getting current time (from CMOS)
- [GDT](gdt.md): Uses the GDT from x86_64 crate
- [PS/2 Controller](ps2.md): Resets the PS/2 controller, needed by PS/2 mouse, but keyboard works by default
- [PS/2 Keyboard](keyboard.md): Parses the scancodes and supported inputs (like python's input())
- [PS/2 Mouse](mouse.md): A PS/2 mouse, but we don't have any use for it


# Inspiration & Sources
### Os's
- [Steelmind OS](https://github.com/daniel-keitel/Steelmind_OS)
- [Redox OS](https://redox-os.org)
- [Hermit OS](https://github.com/hermit-os)
- [Aero](https://github.com/Andy-Python-Programmer/aero)

### Docs
- [An awesome (wiki osdev)](https://wiki.osdev.org/Main_Page)
- Wikipedia has some good resources too
- [Aero's discord server](https://discord.gg/8gwhTTZwt8)
- Started from there: [Write an OS in Rust](https://os.phil-opp.com)
