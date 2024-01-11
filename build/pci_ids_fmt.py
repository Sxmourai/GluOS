with open("pci.ids", mode="r") as f:
    raw_lines = f.readlines()
    lines = list(filter(lambda line: not line.startswith("#") and line.strip()!="", raw_lines))

with open("pci.ids", mode="w") as f:
    f.writelines(lines)