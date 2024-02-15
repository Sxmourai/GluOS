# [ACPI](https://wiki.osdev.org/ACPI)
### How it works
Reads low memory BIOS for [RSDP](https://wiki.osdev.org/RSDP), then gets [RSDT](https://wiki.osdev.org/RSDT), then parses the different memory tables like [FADT](https://wiki.osdev.org/FADT), [MADT](https://wiki.osdev.org/MADT) 

### Required by
- Multiprocessing
- Check if PS/2 controller (see src/drivers/ps2.rs) 
- Computer shutdown
- And many more
