# Hana Control

Nieoficjalny, lekki panel ustawień myszy **Redragon M694 Hana (M694-RGB)** dla Windows.
Ustawia DPI, częstotliwość odświeżania i podświetlenie RGB bez oficjalnego oprogramowania producenta.
Napisany w Rust: jeden plik `.exe` (~5 MB), bez instalatora i bez dodatkowych bibliotek.

![Zrzut ekranu Hana Control](docs/screenshot.png)

> Projekt nie jest powiązany z Redragon. Protokół myszy został ustalony samodzielnie (patrz [Protokół](#protokół)).
> Używasz na własną odpowiedzialność, choć aplikacja zapisuje tylko standardowy blok ustawień i nie dotyka firmware.

## Funkcje

- **Czułość (DPI)** – 5 poziomów jak w instrukcji producenta, wartości od 400 do 7200 DPI, wybór aktywnego poziomu.
- **Częstotliwość odświeżania** – 125 / 250 / 500 / 1000 Hz.
- **Podświetlenie** – efekty z instrukcji (Stały, Oddychanie, Kolorowy ogon, Neon, Fala, Tęczowe oddychanie),
  kolor z presetów lub z koła barw, 4 poziomy jasności, 5 poziomów prędkości, całkowite wyłączenie.
- **Podgląd na żywo** – animowany podgląd podświetlenia na zdjęciu myszy, zanim zapiszesz zmiany.
- **Wszystko na przyciskach** – zmiany trafiają do myszy dopiero po kliknięciu *Zastosuj* i są sprawdzane ponownym odczytem.
- **Kabel i 2.4G** – działa przez odbiornik 2.4 GHz i po kablu USB, sama przełącza się po podłączeniu kabla.
  Ustawienia są wspólne dla obu trybów.
- **Bateria** – poziom naładowania i stan ładowania w oknie oraz w zasobniku systemowym.
- **Powiadomienie o niskiej baterii** – powiadomienie Windows przy 10 %.
- **Praca w tle** – zamknięcie okna chowa aplikację do zasobnika obok zegara; opcjonalny autostart z Windows.
- **Bezpieczeństwo** – przy pierwszym połączeniu zapisuje kopię oryginalnych ustawień myszy, którą można przywrócić;
  przycisk *Domyślne producenta* wczytuje wartości z instrukcji.

## Obsługiwane urządzenia

| Urządzenie | USB VID:PID | Status |
|---|---|---|
| Redragon M694 Hana – odbiornik 2.4G | `320F:226F` | przetestowane |
| Redragon M694 Hana – kabel USB | `320F:226E` | przetestowane |
| Bluetooth | – | nieobsługiwane (brak kanału konfiguracji) |

Inne myszy z układem Telink/Evision (VID `320F`) mogą używać podobnego protokołu, ale nie są obsługiwane.

## Instalacja

1. Pobierz `HanaControl.exe` z zakładki **Releases**.
2. Uruchom – instalacja nie jest potrzebna.

Przy pierwszym uruchomieniu Windows SmartScreen może ostrzec przed nieznanym programem (plik nie jest podpisany cyfrowo):
*Więcej informacji → Uruchom mimo to*.

Ustawienia aplikacji i kopia ustawień myszy są przechowywane w `%APPDATA%\HanaControl`,
autostart w `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

## Budowanie ze źródeł

Wymagania: Windows 10/11 x64, [Rust](https://rustup.rs) (stable, `x86_64-pc-windows-msvc`) i Visual Studio Build Tools (C++).

```bash
cd hana-rs
cargo build --release
```

Gotowy plik: `hana-rs/target/release/hana_control.exe`.

Tryb diagnostyczny powiadomień: `hana_control.exe --test-notify` (wynik w `%TEMP%\hana_notify.txt`).

## Struktura repozytorium

```
hana-rs/                 aplikacja (Rust, egui/eframe + hidapi)
  src/device.rs          komunikacja USB HID z myszą
  src/config.rs          blok ustawień myszy i wartości producenta
  src/worker.rs          wątek w tle: połączenie, bateria, zapis, powiadomienia
  src/app.rs             interfejs
  src/preview.rs         podgląd podświetlenia
  src/tray.rs, win.rs    zasobnik systemowy, autostart, powiadomienia Windows
  src/icons.rs           ikony rysowane wektorowo
  assets/                ikona aplikacji i grafiki podglądu
tools/                   skrypty Python (hidapi) użyte do ustalenia protokołu
docs/                    zrzuty ekranu
```

## Protokół

Mysz komunikuje się przez kolekcję HID producenta **Usage Page `0xFF1C`, Usage `0x92`**, raport `0x04`
(63 bajty danych). Jest to wariant protokołu „Evision” opisanego dla myszy AULA
w projekcie [aulaar](https://github.com/shDavlatbek/aulaar). Pola specyficzne dla M694 zostały potwierdzone testami na myszy.

**Pakiet:** `04 | 00 00 CMD LEN OFF_LO OFF_HI 00 | dane…` – odpowiedź ma ten sam nagłówek, dane od bajtu 7.
Porcja danych: `0x18` przez odbiornik, `0x38` po kablu. Brak sumy kontrolnej.

| Komenda | Znaczenie |
|---|---|
| `0x01` / `0x02` | początek / koniec transakcji |
| `0x05` | odczyt 128-bajtowego bloku ustawień |
| `0x06` | zapis bloku ustawień |
| `0x1A` | bateria: `data[7]` = %, `data[8]` = 0 nie ładuje / 1 ładuje / 2 naładowana |
| `0xAA` | czy mysz jest połączona (`data[6]` po kablu, `data[7]` przez odbiornik) |

**Blok ustawień (128 bajtów):**

| Bajt | Pole |
|---|---|
| `1` | efekt: 0 stały, 1 oddychanie, 2 kolorowy ogon, 3 neon, 4 fala, 5 tęczowe oddychanie, 6 wyłączone |
| `2` | jasność 1–4 |
| `3` | prędkość efektu (mniejsza wartość = szybciej) |
| `6–8` | kolor RGB podświetlenia |
| `10` / `11` | częstotliwość 2.4G / kabel: 0=125, 1=250, 2=500, 3=1000 Hz |
| `12` | aktywny poziom DPI (0–4) |
| `14 + 9·i` | poziom *i*: `+0` włączony, `+2..3` kod czujnika (u16 LE), `+4..5` wartość DPI (opis), `+6..8` kolor wskaźnika |

Czułość ustawia **kod czujnika**, a nie zapisana obok wartość DPI. Fabryczne pary to
400→7, 800→17, 1600→28, 3200→60, 6400→97. Wartości pośrednie aplikacja przelicza interpolacją,
więc są przybliżone.

## Ograniczenia

- Tylko Windows.
- DPI spoza fabrycznych punktów jest przybliżone (brak oficjalnej tabeli kodów czujnika).
- Animacje w podglądzie są symulacją – rzeczywiste tempo i kolory zależą od firmware myszy.
- Gdy działa aplikacja pełnoekranowa, Windows pokazuje powiadomienia tylko w centrum powiadomień (Win+N).

## Podziękowania

- [shDavlatbek/aulaar](https://github.com/shDavlatbek/aulaar) – opis protokołu Evision dla myszy AULA.
- [HeyOkay/HaloBattery](https://github.com/HeyOkay/HaloBattery) – odczyt baterii na tym samym kanale HID.

## Licencja

*Do uzupełnienia przez autora (np. MIT).*
