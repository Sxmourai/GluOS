rm pci.ids -f
wget https://pci-ids.ucw.cz/v2.2/pci.ids.xz
xz -d pci.ids.xz
rm pci.ids.xz -f
python3 pci_ids_fmt.py