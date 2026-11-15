"""Usage: python poke.py IDX=VAL [IDX=VAL ...]  (decimal idx, hex val)  |  python poke.py restore"""
import sys, time
from evision import Mouse
m = Mouse()
if sys.argv[1] == "restore":
    blk = list(bytes.fromhex(open("dumps/01_start.hex").read()))
else:
    blk = m.read_block()
    for a in sys.argv[1:]:
        i, v = a.split("="); i = int(i)
        print(f"[{i}] {blk[i]:02x} -> {int(v,16):02x}"); blk[i] = int(v, 16)
m.write_block(blk); time.sleep(0.3)
print("verify:", "ok" if m.read_block() == blk else "MISMATCH")
m.close()
