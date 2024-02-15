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
- [ACPI](acpi.md)



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
