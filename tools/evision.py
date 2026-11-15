"""Minimal Evision-protocol client (report 4 on vendor collection FF1C:92)."""
import hid, time
VID, PID = 0x320F, 0x226F
L = 0x18  # chunk size via 2.4G dongle

class Mouse:
    def __init__(self):
        global L
        for pid, chunk in ((0x226E, 0x38), (0x226F, 0x18)):  # kabel, potem odbiornik
            ds = [d for d in hid.enumerate(VID, pid) if d['usage_page'] == 0xFF1C]
            if not ds: continue
            self.h = hid.device(); self.h.open_path(ds[0]['path']); self.h.set_nonblocking(True)
            L = chunk
            try:
                r = self.xfer([0, 0, 0xAA, L, 0, 0], wait=0.5)
                if r[6] | r[7]: return
            except TimeoutError: pass
            self.h.close()
        raise RuntimeError("Nie znaleziono myszy M694")
    def close(self): self.h.close()
    def xfer(self, data, wait=1.0):
        while self.h.read(64): pass
        self.h.write([0x04] + data + [0] * (63 - len(data)))
        t = time.time()
        while time.time() - t < wait:
            r = self.h.read(64)
            if r and r[0] == 0x04 and r[3] == data[2]: return r[1:]
            time.sleep(0.005)
        raise TimeoutError(hex(data[2]))
    def battery(self): return self.xfer([0, 0, 0x1A, L, 0, 0])[7]
    def read_block(self):
        self.xfer([0, 0, 0x01, L, 0, 0])
        blk = []
        for off in range(0, 128, L):
            n = min(L, 128 - off)
            blk += list(self.xfer([0, 0, 0x05, n, off, 0, 0])[7:7 + n])
        self.xfer([0, 0, 0x02, L, 0, 0])
        return blk

    def write_block(self, blk):
        assert len(blk) == 128
        self.xfer([0, 0, 0x01, L, 0, 0])
        for off in range(0, 128, L):
            n = min(L, 128 - off)
            self.xfer([0, 0, 0x06, n, off, 0, 0] + list(blk[off:off + n]))
            time.sleep(0.02)
        self.xfer([0, 0, 0x02, L, 0, 0])
