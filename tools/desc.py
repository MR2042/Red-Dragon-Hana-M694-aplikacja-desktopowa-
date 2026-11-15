import hid
for d in hid.enumerate(0x320F, 0x226F):
    if d['usage_page'] < 0xFF00: continue
    h = hid.device(); h.open_path(d['path'])
    try:
        rd = h.get_report_descriptor()
        print(f"up=0x{d['usage_page']:04X} if={d['interface_number']}:", bytes(rd).hex(' '))
    except Exception as e:
        print("err", e)
    h.close()
