"""Write the block exactly as read, then verify read-back is identical."""
from evision import Mouse
m = Mouse()
a = m.read_block()
m.write_block(a)
import time; time.sleep(0.3)
b = m.read_block()
m.close()
print("identical" if a == b else f"DIFF at {[i for i in range(128) if a[i] != b[i]]}")
