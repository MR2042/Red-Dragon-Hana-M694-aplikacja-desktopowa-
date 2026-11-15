"""Usage: python dump.py <label>   -> saves dumps/<n>_<label>.hex and shows diff vs previous dump."""
import sys, os, glob
from evision import Mouse
os.makedirs("dumps", exist_ok=True)
m = Mouse(); blk = m.read_block(); m.close()
prev = sorted(glob.glob("dumps/*.hex"))
n = len(prev) + 1
name = f"dumps/{n:02d}_{sys.argv[1] if len(sys.argv) > 1 else 'dump'}.hex"
open(name, "w").write(bytes(blk).hex(' '))
print("saved", name)
if prev:
    old = bytes.fromhex(open(prev[-1]).read())
    diffs = [(i, old[i], blk[i]) for i in range(128) if old[i] != blk[i]]
    print(f"diff vs {prev[-1]}:", "none" if not diffs else "")
    for i, a, b in diffs: print(f"  [{i:3d}] 0x{i:02x}: {a:02x} -> {b:02x}  ({a} -> {b})")
