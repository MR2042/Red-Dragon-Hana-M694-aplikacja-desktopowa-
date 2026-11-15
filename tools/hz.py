"""Measure mouse report rate (Hz) via Windows Raw Input. Move the mouse in circles for the test duration.
Usage: python hz.py [seconds]"""
import ctypes, ctypes.wintypes as w, sys, time

u32 = ctypes.WinDLL("user32", use_last_error=True)
k32 = ctypes.WinDLL("kernel32")
WM_INPUT = 0x00FF
RIDEV_INPUTSINK = 0x100
RID_HEADER = 0x10000005
WNDPROC = ctypes.WINFUNCTYPE(ctypes.c_ssize_t, w.HWND, w.UINT, w.WPARAM, w.LPARAM)

class RAWINPUTDEVICE(ctypes.Structure):
    _fields_ = [("usUsagePage", w.USHORT), ("usUsage", w.USHORT), ("dwFlags", w.DWORD), ("hwndTarget", w.HWND)]

class RAWINPUTHEADER(ctypes.Structure):
    _fields_ = [("dwType", w.DWORD), ("dwSize", w.DWORD), ("hDevice", w.HANDLE), ("wParam", w.WPARAM)]

class WNDCLASS(ctypes.Structure):
    _fields_ = [("style", w.UINT), ("lpfnWndProc", WNDPROC), ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int),
                ("hInstance", w.HINSTANCE), ("hIcon", w.HICON), ("hCursor", w.HANDLE), ("hbrBackground", w.HBRUSH),
                ("lpszMenuName", w.LPCWSTR), ("lpszClassName", w.LPCWSTR)]

u32.DefWindowProcW.argtypes = [w.HWND, w.UINT, w.WPARAM, w.LPARAM]
u32.DefWindowProcW.restype = ctypes.c_ssize_t
u32.GetRawInputData.argtypes = [w.HANDLE, w.UINT, ctypes.c_void_p, ctypes.POINTER(w.UINT), w.UINT]
u32.CreateWindowExW.restype = w.HWND
u32.CreateWindowExW.argtypes = [w.DWORD, w.LPCWSTR, w.LPCWSTR, w.DWORD, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                                ctypes.c_int, w.HWND, w.HMENU, w.HINSTANCE, w.LPVOID]

counts = {}  # hDevice -> list of timestamps

def proc(hwnd, msg, wp, lp):
    if msg == WM_INPUT:
        hdr = RAWINPUTHEADER(); sz = w.UINT(ctypes.sizeof(hdr))
        u32.GetRawInputData(lp, RID_HEADER, ctypes.byref(hdr), ctypes.byref(sz), ctypes.sizeof(RAWINPUTHEADER))
        counts.setdefault(hdr.hDevice or 0, []).append(time.perf_counter())
    return u32.DefWindowProcW(hwnd, msg, wp, lp)

wndproc = WNDPROC(proc)
wc = WNDCLASS(lpfnWndProc=wndproc, hInstance=k32.GetModuleHandleW(None), lpszClassName="HzMeter")
u32.RegisterClassW(ctypes.byref(wc))
hwnd = u32.CreateWindowExW(0, "HzMeter", "", 0, 0, 0, 0, 0, None, None, wc.hInstance, None)
rid = RAWINPUTDEVICE(1, 2, RIDEV_INPUTSINK, hwnd)
u32.RegisterRawInputDevices(ctypes.byref(rid), 1, ctypes.sizeof(rid))

dur = float(sys.argv[1]) if len(sys.argv) > 1 else 6
print(f"Rusz myszką w kółko przez {dur:.0f} s...", flush=True)
msg = w.MSG(); end = time.perf_counter() + dur
while time.perf_counter() < end:
    while u32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
        u32.TranslateMessage(ctypes.byref(msg)); u32.DispatchMessageW(ctypes.byref(msg))
    time.sleep(0.0005)

for dev, ts in counts.items():
    if len(ts) < 20: continue
    gaps = sorted(b - a for a, b in zip(ts, ts[1:]))
    med = gaps[len(gaps) // 2]
    print(f"urządzenie {dev}: {len(ts)} raportów, mediana odstępu {med*1000:.2f} ms -> ok. {1/med:.0f} Hz")
if not any(len(t) >= 20 for t in counts.values()):
    print("Za mało ruchu - spróbuj jeszcze raz.")
