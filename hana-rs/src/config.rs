//! 128-bajtowy blok konfiguracji myszy i wartości dopuszczone przez producenta.
//! Wszystkie pola poniżej zostały potwierdzone testami na M694.

pub const CONFIG_SIZE: usize = 128;
/// Instrukcja producenta: 5 poziomów DPI.
pub const STAGES: usize = 5;
const STAGE_BASE: usize = 14;
const STAGE_STRIDE: usize = 9;

pub type Rgb = [u8; 3];

/// Częstotliwość raportowania wg instrukcji: 125 / 250 / 500 / 1000 Hz (indeksy 0–3).
pub const POLLING_HZ: [u16; 4] = [125, 250, 500, 1000];

/// Efekty z instrukcji producenta (kod w bajcie 1) – kolejność jak w instrukcji.
pub const EFFECTS: [(u8, &str); 6] = [
    (0, "Stały"),
    (1, "Oddychanie"),
    (2, "Kolorowy ogon"),
    (3, "Neon"),
    (4, "Fala"),
    (5, "Tęczowe oddychanie"),
];
pub const EFFECT_OFF: u8 = 6;
/// Efekty, w których wybiera się kolor (pozostałe są wielokolorowe).
pub fn effect_uses_color(code: u8) -> bool {
    code <= 1
}

/// Wartości DPI do wyboru: zakres producenta do 7200, w tym wszystkie wartości z instrukcji.
pub const DPI_VALUES: [u16; 12] = [400, 800, 1000, 1200, 1600, 2000, 2400, 3200, 4000, 4800, 6400, 7200];

/// Domyślne poziomy z instrukcji: kolor wskaźnika i DPI.
pub const FACTORY_STAGES: [(u16, Rgb); STAGES] = [
    (800, [255, 0, 0]),     // czerwony
    (1200, [0, 0, 255]),    // niebieski
    (1600, [0, 255, 0]),    // zielony
    (3200, [255, 0, 255]),  // fioletowy
    (7200, [255, 255, 0]),  // żółty
];
/// Domyślne podświetlenie z instrukcji: stałe, niebieskie.
pub const FACTORY_LIGHT: Rgb = [0, 0, 255];

/// Podstawowe kolory (presety). Dowolny inny kolor wybiera się na kole barw.
pub const PALETTE: [(Rgb, &str); 8] = [
    ([255, 0, 0], "Czerwony"),
    ([255, 96, 0], "Pomarańczowy"),
    ([255, 255, 0], "Żółty"),
    ([0, 255, 0], "Zielony"),
    ([0, 255, 255], "Turkusowy"),
    ([0, 0, 255], "Niebieski"),
    ([255, 0, 255], "Fioletowy"),
    ([255, 255, 255], "Biały"),
];

/// Jasność 1–4 (0 gasi diody – do tego służy osobny przycisk).
pub const BRIGHTNESS_LEVELS: [u8; 4] = [1, 2, 3, 4];
/// Prędkość efektu: 5 poziomów; w myszy mniejsza wartość = szybciej.
pub const SPEED_RAW: [u8; 5] = [82, 62, 42, 22, 1];

/// Przeliczanie DPI na kod czujnika – punkty z fabrycznej konfiguracji myszy,
/// między nimi interpolacja liniowa, poza nimi przedłużenie skrajnych odcinków.
const DPI_POINTS: [(f64, f64); 5] = [(400.0, 7.0), (800.0, 17.0), (1600.0, 28.0), (3200.0, 60.0), (6400.0, 97.0)];

pub fn dpi_to_code(dpi: u16) -> u16 {
    let d = dpi as f64;
    let mut seg = 0;
    while seg < DPI_POINTS.len() - 2 && d > DPI_POINTS[seg + 1].0 {
        seg += 1;
    }
    let (d0, c0) = DPI_POINTS[seg];
    let (d1, c1) = DPI_POINTS[seg + 1];
    (c0 + (d - d0) * (c1 - c0) / (d1 - d0)).round().max(1.0) as u16
}

#[derive(Clone, PartialEq)]
pub struct Config {
    pub raw: [u8; CONFIG_SIZE],
}

impl Config {
    fn s(i: usize) -> usize {
        STAGE_BASE + i * STAGE_STRIDE
    }

    pub fn effect(&self) -> u8 { self.raw[1] }
    pub fn set_effect(&mut self, v: u8) { self.raw[1] = v; }
    pub fn brightness(&self) -> u8 { self.raw[2] }
    pub fn set_brightness(&mut self, v: u8) { self.raw[2] = v; }
    pub fn speed_raw(&self) -> u8 { self.raw[3] }
    pub fn set_speed_raw(&mut self, v: u8) { self.raw[3] = v; }
    pub fn light_color(&self) -> Rgb { [self.raw[6], self.raw[7], self.raw[8]] }
    pub fn set_light_color(&mut self, c: Rgb) { self.raw[6..9].copy_from_slice(&c); }

    /// Indeks częstotliwości (0–3). Zapisujemy go dla 2.4G i dla kabla.
    pub fn polling(&self) -> u8 { self.raw[10].min(3) }
    pub fn set_polling(&mut self, v: u8) {
        self.raw[10] = v;
        self.raw[11] = v;
    }

    pub fn active_stage(&self) -> usize { (self.raw[12] as usize).min(STAGES - 1) }
    pub fn set_active_stage(&mut self, i: usize) { self.raw[12] = i as u8; }

    pub fn stage_enabled(&self, i: usize) -> bool { self.raw[Self::s(i)] != 0 }
    pub fn set_stage_enabled(&mut self, i: usize, v: bool) { self.raw[Self::s(i)] = v as u8; }

    /// DPI poziomu (opis zapisany obok kodu czujnika).
    pub fn stage_dpi(&self, i: usize) -> u16 {
        let s = Self::s(i);
        u16::from_le_bytes([self.raw[s + 4], self.raw[s + 5]])
    }
    pub fn set_stage_dpi(&mut self, i: usize, dpi: u16) {
        let s = Self::s(i);
        self.raw[s + 2..s + 4].copy_from_slice(&dpi_to_code(dpi).to_le_bytes());
        self.raw[s + 4..s + 6].copy_from_slice(&dpi.to_le_bytes());
    }

    pub fn stage_color(&self, i: usize) -> Rgb {
        let s = Self::s(i) + 6;
        [self.raw[s], self.raw[s + 1], self.raw[s + 2]]
    }
    pub fn set_stage_color(&mut self, i: usize, c: Rgb) {
        let s = Self::s(i) + 6;
        self.raw[s..s + 3].copy_from_slice(&c);
    }

    /// Ustawienia domyślne z instrukcji (DPI, kolory, podświetlenie). Pozostałe pola bez zmian.
    pub fn apply_factory(&mut self) {
        for (i, (dpi, color)) in FACTORY_STAGES.iter().enumerate() {
            self.set_stage_enabled(i, true);
            self.set_stage_dpi(i, *dpi);
            self.set_stage_color(i, *color);
        }
        self.set_active_stage(0);
        self.set_effect(0);
        self.set_light_color(FACTORY_LIGHT);
        self.set_brightness(4);
    }
}
