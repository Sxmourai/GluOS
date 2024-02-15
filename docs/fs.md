# Filesystems
### Requires
- Disk

### How it works
Reads some infos on the disk to identify the partitions on the disk
There can be 2 types of headers:
- [MBR](https://wiki.osdev.org/MBR_(x86))
- [GPT](https://wiki.osdev.org/GPT)
Then, we try to initialise a filesystem on the partition (see the different implementation to find out how they work)


### Required by
- ELF loading
- User will want to read his files

### Working on
- BTRFS
- A better abstraction on files (maybe a [VFS](https://wiki.osdev.org/VFS))
