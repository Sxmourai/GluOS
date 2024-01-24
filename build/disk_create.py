# Example: python3 disk_create.py fat-disk.img 30M -format fat32
import sys,os
def cmd(command):
    print(">",command)
    os.system(command)
def get(list, idx, default=None):
  try:
    return list[idx]
  except IndexError:
    return default
import argparse

filesystems = {
        "fat32": "mkfs.fat -F 32",
        "fat16": "mkfs.fat -F 16",
        "fat12": "mkfs.fat -F 12",
        "ext4": "mkfs.ext4",
        "ext3": "mkfs.ext3",
        "ext2": "mkfs.ext2",
}

if __name__ == "__main__":
    parser = argparse.ArgumentParser("Disk creator", description="Creates disk to test my kernel !")
    parser.add_argument("filename", default="disk.img")
    parser.add_argument("size",)
    parser.add_argument("-format")
    parser.add_argument('-partition', action='store_true')
    parser.add_argument('-header_type', default="gpt")
    args = parser.parse_args(sys.argv[1:])
    if not args.filename.endswith(".img"):args.filename+=".img"
    if args.format.lower() in filesystems.keys():
        format = filesystems[args.format.lower()]
    else:
        print(f"Invalid format type: {list(filesystems.keys())}")
        exit(1)
    
    print("\n\tCreating raw disk with QEMU")
    cmd(f"qemu-img create -f raw {args.filename} {args.size}")
    if args.partition:
        print(f"\n\tFormatting disk to {format}")
        cmd(f"{format} {args.filename}")
    else:
        print(f"\n\tCreating label on disk ({args.header_type})")
        cmd(f"parted {args.filename} mklabel {args.header_type} --script")
        print(f"\n\tCreating partition")
        cmd(fr"parted {args.filename} mkpart primary {args.format} 0% 100% --script")
        print(f"\n\tMounting partition on loop device")
        cmd(fr"sudo losetup -o 512 /dev/loop3 {args.filename}")
        print(f"\n\tCreating fs on partition")
        cmd(fr"sudo {format} /dev/loop3") # Sudo because sometimes it's needed
        print(f"\n\tMounting partition")
        cmd(fr"sudo mount /dev/loop3 mounted_disk")
        print(f"\n\tCreating a test file")
        cmd(fr"echo hi | sudo tee -a mounted_disk/hello.txt")
        print(f"\n\tCreating a test dir")
        cmd(fr"sudo mkdir mounted_disk/hello_dirs")
        cmd(fr"echo hello | sudo tee -a mounted_disk/hello_dirs/second_hello_wewe.txt")
        print(f"\n\tUnmounting partition")
        cmd(fr"sudo umount mounted_disk")
        print(f"\n\tUnmounting partition from loop device")
        cmd(fr"sudo losetup -d /dev/loop3")
        

    drives = []

    for ele in os.listdir("."):
        if ele.endswith(".img"):
            drives.append(f'"-drive", "file=build/{ele},format=raw",')

    with open("../Cargo.toml", "r") as f:
        lines = f.readlines()
        start = 0
        for i,line in enumerate(lines[:]):
            line = line.lstrip().rstrip()
            if line.startswith("# ENDDISKFLAG"):
                break
            if start!=0:
                lines[i] = ""
            if line.startswith("# DISKFLAG"):
                start = i
            
                
        for i,line in enumerate(lines[:]):
            line = line.lstrip().rstrip()
            if line.startswith("# DISKFLAG"):
                for j,drive in enumerate(drives):
                    lines.insert(i+j+1, drive+"\n")
                
    with open("../Cargo.toml", "w") as f:
        f.writelines(lines)
    