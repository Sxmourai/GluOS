# Example: python3 disk_create.py fat-disk.img 30M -format fat32
import sys,os

def cmd(command, description):
    print("\n\t", description)
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
        "ntfs": "mkfs.ntfs",
}

def main(args):
    parser = argparse.ArgumentParser("Disk creator", description="Creates disk to test my kernel !")
    parser.add_argument("filename", default="disk.img")
    parser.add_argument("size",)
    parser.add_argument("-format")
    parser.add_argument('-partition', action='store_true')
    parser.add_argument('-header_type', default="gpt")
    args = parser.parse_args(args)
    if not args.filename.endswith(".img"):args.filename+=".img"
    if args.format.lower() in filesystems.keys():
        format = filesystems[args.format.lower()]
    else:
        print(f"Invalid format type: {list(filesystems.keys())}")
        exit(1)
    
    cmd(f"qemu-img create -f raw {args.filename} {args.size}", "Creating raw disk with QEMU")
    if args.partition:
        cmd(f"{format} {args.filename}", f"Formatting disk to {format}")
    else:
        cmd(f"parted {args.filename} mklabel {args.header_type} --script", f"Creating label on disk ({args.header_type})")
        cmd(fr"parted {args.filename} mkpart primary {args.format} 18432B 100% --script", f"Creating partition")
        cmd(fr"sudo losetup -o 18432 /dev/loop3 {args.filename}", "Mounting partition on loop device")
        if "ntfs" in format:
            print("Ouhh NTFS, good luck =)")
            format += " -F "
        cmd(fr"sudo {format} /dev/loop3", "Creating fs on partition") # Sudo because sometimes it's needed
        cmd(fr"sudo mount /dev/loop3 mounted_disk", "Mounting partition")
        cmd(fr"echo hi | sudo tee -a mounted_disk/hello.txt", "Creating a test file")
        cmd(fr"sudo mkdir mounted_disk/hello_dirs", "Creating a test dir")
        cmd(fr"echo hello | sudo tee -a mounted_disk/hello_dirs/second_hello_wewe.txt", "Creating another test file in the folder")
        cmd(fr"sudo cp userland.o mounted_disk/userland.o", "Copying a simple executable file")
        cmd(fr"sudo umount mounted_disk", "Unmounting partition")
        cmd(fr"sudo losetup -d /dev/loop3", "Unmounting partition from loop device")

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
    
DISKS = [
    "fatgpt 25M -format fat32",
    "ext2gpt 30M -format ext2",
    "ntfs 100M -format NTFS",
]

if __name__ == "__main__":
    if "create-all-disks" in sys.argv:
        for disk in DISKS:
            main(disk.split(" "))
    else:
        main(sys.argv[1:])