import pr

barcode = "C4533055014213025"
plid = pr.get_plid(barcode)
print("PLID:",[f"0x{b:02x}" for b in plid])
