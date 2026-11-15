"""Read-only probe of the Evision protocol (report 4 on FF1C:92). Sends NO writes to config."""
import hid, time, sys
VID, PID = 0x320F, 0x226F
path = next(d['path'] for d in hid.enumerate(VID, PID) if d['usage_page'] == 0xFF1C)
h = hid.device(); h.open_path(path); h.set_nonblocking(True)
L = 0x18

def drain():
    while h.read(64): pass

def xfer(data, wait=1.0):
    drain()
    pkt = [0x04] + data + [0] * (63 - len(data))
    h.write(pkt)
    t = time.time()
    while time.time() - t < wait:
        r = h.read(64)
        if r:
            if r[0] == 0x04: return r[1:]
            print("   (other report)", bytes(r).hex(' '))
        time.sleep(0.005)
    return None

def show(name, r):
    print(f"{name:10}", bytes(r[:32]).hex(' ') if r else "TIMEOUT")

show("online", xfer([0, 0, 0xAA, L, 0, 0]))
show("begin", xfer([0, 0, 0x01, L, 0, 0]))
show("info03", xfer([0, 0, 0x03, 0x0A, 0, 0]))
show("battery", xfer([0, 0, 0x1A, L, 0, 0]))
show("rxfw", xfer([0, 0, 0xB3, L, 0, 0]))
blk = []
for off in range(0, 128, L):
    size = min(L, 128 - off)
    r = xfer([0, 0, 0x05, size, off, 0, 0])
    show(f"read@{off:02x}", r)
    if r: blk += list(r[7:7 + size])
show("end", xfer([0, 0, 0x02, L, 0, 0]))
print("\nBLOCK", len(blk))
for i in range(0, len(blk), 16): print(f"{i:3d}: " + bytes(blk[i:i+16]).hex(' '))
h.close()
