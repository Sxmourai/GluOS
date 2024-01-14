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
    args = parser.parse_args(sys.argv[1:])
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
        print(f"\n\tCreating label on disk (gpt)")
        cmd(f"parted {args.filename} mklabel gpt --script")
        print(f"\n\tFormating disk")
        cmd(fr"parted {args.filename} mkpart primary {args.format} 0% 100% --script")

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
    