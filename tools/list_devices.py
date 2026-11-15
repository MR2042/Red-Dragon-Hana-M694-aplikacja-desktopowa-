import hid
for d in hid.enumerate(0x320F, 0x226F):
    print(f"if={d['interface_number']} up=0x{d['usage_page']:04X} u=0x{d['usage']:04X} prod={d['product_string']!r} mfr={d['manufacturer_string']!r} rel=0x{d['release_number']:04X}")
    print("   ", d['path'].decode())
