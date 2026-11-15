"""Interaktywny test kodów efektów świetlnych. Na końcu przywraca ustawienia sprzed testu."""
import time
from evision import Mouse
m = Mouse()
orig = m.read_block()
tests = [(f"kod efektu {c}", {1: c, 2: 4}) for c in range(0, 8)] + [("jasność 0 (efekt Stały)", {1: 0, 2: 0})]
print("Dla każdego kroku zapisz, co robi podświetlenie myszy, i naciśnij Enter.\n")
try:
    for i, (name, patch) in enumerate(tests, 1):
        blk = list(orig)
        blk[6:9] = [0, 0, 255]  # kolor niebieski, żeby było widać efekty jednokolorowe
        blk[3] = 0x30           # średnia prędkość
        for k, v in patch.items(): blk[k] = v
        m.write_block(blk)
        input(f"[{i}/{len(tests)}] {name}  -> obserwuj mysz, potem Enter ")
finally:
    m.write_block(orig)
    m.close()
    print("\nPrzywrócono ustawienia sprzed testu.")
