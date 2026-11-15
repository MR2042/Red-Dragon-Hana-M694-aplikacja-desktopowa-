use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontId, Layout, Margin, RichText, Sense, Stroke, Vec2,
};

use crate::config::*;
use crate::device::Battery;
use crate::worker::{self, Event, Request};

// ---------- kolory motywu ----------
const BG: Color32 = Color32::from_rgb(0x13, 0x15, 0x19);
const CARD: Color32 = Color32::from_rgb(0x22, 0x26, 0x2E);
const CARD_HI: Color32 = Color32::from_rgb(0x2C, 0x31, 0x3B);
const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xEE);
const MUTED: Color32 = Color32::from_rgb(0x8C, 0x93, 0xA1);
const ACCENT: Color32 = Color32::from_rgb(0xE0, 0x2F, 0x44);
const GOOD: Color32 = Color32::from_rgb(0x2E, 0xCC, 0x71);

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn backup_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("HanaControl").join("kopia_oryginalna.bin"))
}

pub struct HanaApp {
    tx: Sender<Request>,
    rx: Receiver<Event>,
    /// Konfiguracja aktualnie zapisana w myszy.
    base: Option<Config>,
    /// Konfiguracja edytowana w oknie.
    edit: Option<Config>,
    wired: bool,
    battery: Option<Battery>,
    status: String,
    busy: bool,
    stage: usize,
    /// Efekt do przywrócenia po ponownym włączeniu podświetlenia.
    last_effect: u8,
    /// Ile klatek jeszcze chować okno przy starcie z autostartu.
    hide_frames: u8,
    /// Czy pokazano już podpowiedź, że aplikacja działa w zasobniku.
    tray_hint_shown: bool,
    autostart: bool,
    preview: crate::preview::Preview,
    /// Czy otwarty jest ekran ustawień aplikacji.
    settings_open: bool,
    /// Czy rozwinięte jest koło barw (kolor własny).
    wheel_open: bool,
    /// Wynik ostatniego testowego powiadomienia (tekst, czy sukces).
    notify_result: Option<(String, bool)>,
}

impl HanaApp {
    pub fn new(cc: &eframe::CreationContext, start_hidden: bool) -> Self {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        if let Ok(h) = cc.window_handle() {
            if let RawWindowHandle::Win32(w) = h.as_raw() {
                crate::win::set_hwnd(w.hwnd.get());
            }
        }
        setup_style(&cc.egui_ctx);
        let tray = crate::tray::spawn(cc.egui_ctx.clone());
        let (tx, rx) = worker::spawn(cc.egui_ctx.clone(), tray);
        Self {
            hide_frames: if start_hidden { 4 } else { 0 },
            tray_hint_shown: start_hidden,
            autostart: crate::win::autostart_enabled(),
            preview: crate::preview::Preview::new(&cc.egui_ctx),
            settings_open: false,
            wheel_open: false,
            notify_result: None,
            tx,
            rx,
            base: None,
            edit: None,
            wired: false,
            battery: None,
            status: "Łączenie z myszą…".into(),
            busy: true,
            stage: 0,
            last_effect: 0,
        }
    }

    fn connected(&self) -> bool {
        self.edit.is_some()
    }

    fn dirty(&self) -> bool {
        matches!((&self.base, &self.edit), (Some(b), Some(e)) if b != e)
    }

    fn handle_events(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Connected { wired, config, battery } => {
                    save_backup_once(&config);
                    // Niezapisane zmiany zostają w oknie także po zmianie kabla/odbiornika.
                    if self.edit.is_none() || !self.dirty() {
                        self.edit = Some(config.clone());
                        self.stage = config.active_stage();
                    }
                    if config.effect() < EFFECT_OFF {
                        self.last_effect = config.effect();
                    }
                    self.base = Some(config);
                    self.wired = wired;
                    self.battery = battery;
                    self.busy = false;
                    self.status = "Ustawienia odczytane z myszy.".into();
                }
                Event::Disconnected(msg) => {
                    self.base = None;
                    self.edit = None;
                    self.battery = None;
                    self.busy = false;
                    self.status = msg;
                }
                Event::Battery(b) => self.battery = Some(b),
                Event::Written { config, verified } => {
                    self.edit = Some(config.clone());
                    self.base = Some(config);
                    self.busy = false;
                    self.status = if verified {
                        "Zapisano – ustawienia są w pamięci myszy.".into()
                    } else {
                        "Zapisano, ale odczyt kontrolny się różni. Sprawdź ustawienia.".into()
                    };
                }
                Event::Error(msg) => {
                    self.busy = false;
                    self.status = msg;
                }
            }
        }
    }

    fn send(&mut self, req: Request, status: &str) {
        self.busy = true;
        self.status = status.into();
        let _ = self.tx.send(req);
    }
}

fn save_backup_once(cfg: &Config) {
    if let Some(p) = backup_path() {
        if !p.exists() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let _ = std::fs::write(p, cfg.raw);
        }
    }
}

fn load_backup() -> Option<Config> {
    let data = std::fs::read(backup_path()?).ok()?;
    let raw: [u8; CONFIG_SIZE] = data.try_into().ok()?;
    Some(Config { raw })
}

// ---------- wygląd ----------
fn setup_style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = BG;
    v.override_text_color = Some(TEXT);
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::NONE;
    v.widgets.hovered.weak_bg_fill = CARD_HI;
    v.widgets.hovered.bg_stroke = Stroke::NONE;
    v.widgets.active.weak_bg_fill = ACCENT;
    v.widgets.active.bg_stroke = Stroke::NONE;
    v.selection.bg_fill = ACCENT;
    ctx.set_visuals(v);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.5));
    style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.5));
    style.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(20.0));
    ctx.set_style(style);
}

/// Sekcja bez ramki: mały nagłówek i treść – cała aplikacja jest jedną płaszczyzną.
fn section(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.label(RichText::new(title).size(12.5).strong().color(MUTED));
    ui.add_space(4.0);
    body(ui);
    ui.add_space(26.0);
}

fn sublabel(ui: &mut egui::Ui, text: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(text).size(13.0).color(MUTED));
}

/// Płaski przycisk wyboru: czerwony, gdy wybrany.
fn choice(ui: &mut egui::Ui, selected: bool, text: &str, min_w: f32) -> bool {
    let (fill, color) = if selected { (ACCENT, Color32::WHITE) } else { (CARD, TEXT) };
    ui.add(
        egui::Button::new(RichText::new(text).color(color).strong())
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(7))
            .min_size(Vec2::new(min_w, 36.0)),
    )
    .clicked()
}

/// Przełącznik wł./wył.
fn toggle(ui: &mut egui::Ui, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), Sense::click());
    let t = ui.ctx().animate_bool(resp.id, on);
    let p = ui.painter();
    let track = if on { ACCENT } else { CARD_HI };
    p.rect_filled(rect, CornerRadius::same(12), track);
    let x = egui::lerp(rect.left() + 12.0..=rect.right() - 12.0, t);
    p.circle_filled(egui::pos2(x, rect.center().y), 9.0, Color32::WHITE);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

/// Kółko z kolorem do wyboru.
fn swatch(ui: &mut egui::Ui, color: Rgb, name: &str, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.circle_stroke(rect.center(), 15.5, Stroke::new(2.5_f32, Color32::WHITE));
    } else if resp.hovered() {
        p.circle_stroke(rect.center(), 15.5, Stroke::new(1.5_f32, MUTED));
    }
    p.circle_filled(rect.center(), 12.0, rgb(color));
    resp.on_hover_text(name).clicked()
}

/// Presety kolorów + tęczowe kółko, które rozwija koło barw.
/// Zwraca wybrany preset i informację, czy kliknięto tęczowe kółko.
fn palette(ui: &mut egui::Ui, current: Rgb, wheel_open: bool) -> (Option<Rgb>, bool) {
    let mut picked = None;
    let mut rainbow = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (c, name) in PALETTE {
            if swatch(ui, c, name, c == current && !wheel_open) {
                picked = Some(c);
            }
        }
        // tęczowe kółko: kolor własny z koła barw
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
        let custom = !PALETTE.iter().any(|(c, _)| *c == current);
        if wheel_open || custom {
            ui.painter().circle_stroke(rect.center(), 15.5, Stroke::new(2.5_f32, Color32::WHITE));
        } else if resp.hovered() {
            ui.painter().circle_stroke(rect.center(), 15.5, Stroke::new(1.5_f32, MUTED));
        }
        crate::icons::rainbow_circle(ui.painter(), rect.center(), 12.0);
        if resp.on_hover_text("Kolor własny – koło barw").clicked() {
            rainbow = true;
        }
    });
    (picked, rainbow)
}

// ---------- koło barw ----------
pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [(r * 255.0).round() as u8, (g * 255.0).round() as u8, (b * 255.0).round() as u8]
}

fn rgb_to_hs(c: Rgb) -> (f32, f32) {
    let [r, g, b] = c.map(|x| x as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    if max <= 0.0 || d <= 0.0 {
        return (0.0, 0.0);
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    (h, d / max)
}

/// Koło barw: kąt = odcień, odległość od środka = nasycenie (jasność ustawia się osobno).
fn color_wheel(ui: &mut egui::Ui, current: Rgb, size: f32) -> Option<Rgb> {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
    let c = rect.center();
    let radius = size / 2.0 - 4.0;
    let painter = ui.painter_at(rect.expand(2.0));

    // siatka: pierścienie od środka (biel) do krawędzi (pełne nasycenie)
    let (rings, segs) = (10usize, 72usize);
    let mut mesh = egui::epaint::Mesh::default();
    let dim = !ui.is_enabled();
    let col = |h: f32, s: f32| {
        let [r, g, b] = hsv_to_rgb(h, s, 1.0);
        let k = if dim { 0.35 } else { 1.0 };
        Color32::from_rgb((r as f32 * k) as u8, (g as f32 * k) as u8, (b as f32 * k) as u8)
    };
    mesh.colored_vertex(c, col(0.0, 0.0));
    for ring in 1..=rings {
        let s = ring as f32 / rings as f32;
        for j in 0..segs {
            let h = j as f32 / segs as f32;
            let a = h * std::f32::consts::TAU;
            mesh.colored_vertex(c + Vec2::new(a.cos(), a.sin()) * radius * s, col(h, s));
        }
    }
    let idx = |ring: usize, j: usize| -> u32 { if ring == 0 { 0 } else { (1 + (ring - 1) * segs + j % segs) as u32 } };
    for j in 0..segs {
        mesh.add_triangle(0, idx(1, j), idx(1, j + 1));
    }
    for ring in 1..rings {
        for j in 0..segs {
            let (a, b, cc, d) = (idx(ring, j), idx(ring, j + 1), idx(ring + 1, j + 1), idx(ring + 1, j));
            mesh.add_triangle(a, b, cc);
            mesh.add_triangle(a, cc, d);
        }
    }
    painter.add(egui::Shape::mesh(mesh));

    // wskaźnik bieżącego koloru
    let (h, s) = rgb_to_hs(current);
    let a = h * std::f32::consts::TAU;
    let m = c + Vec2::new(a.cos(), a.sin()) * radius * s;
    painter.circle_stroke(m, 7.0, Stroke::new(3.0_f32, Color32::from_black_alpha(160)));
    painter.circle_stroke(m, 7.0, Stroke::new(2.0_f32, Color32::WHITE));

    // wybór kliknięciem lub przeciąganiem
    if resp.is_pointer_button_down_on() {
        if let Some(p) = resp.interact_pointer_pos() {
            let d = p - c;
            let s = (d.length() / radius).clamp(0.0, 1.0);
            let h = d.y.atan2(d.x) / std::f32::consts::TAU;
            return Some(hsv_to_rgb(h, s, 1.0));
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::Crosshair);
    None
}

impl eframe::App for HanaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();

        // Start z autostartu: eframe pokazuje okno po pierwszej klatce, więc chowamy je przez kilka klatek.
        if self.hide_frames > 0 {
            self.hide_frames -= 1;
            crate::win::hide_window();
            ctx.request_repaint();
        }

        // Przycisk X chowa okno do zasobnika zamiast zamykać aplikację.
        if ctx.input(|i| i.viewport().close_requested()) {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            crate::win::hide_window();
            if !self.tray_hint_shown {
                self.tray_hint_shown = true;
                crate::win::notify(
                    "Hana Control działa w tle",
                    "Aplikacja jest w zasobniku obok zegara. Aby ją zamknąć, kliknij ikonę prawym przyciskiem → Zamknij aplikację.",
                );
            }
        }

        self.header(ctx);
        if !self.settings_open {
            self.footer(ctx);
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BG).inner_margin(Margin::symmetric(28, 22)))
            .show(ctx, |ui| {
                if self.settings_open {
                    self.settings_page(ui);
                    return;
                }
                let Some(mut cfg) = self.edit.clone() else {
                    self.no_device(ui);
                    return;
                };
                egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.columns(3, |cols| {
                        self.dpi_section(&mut cols[0], &mut cfg);
                        self.polling_section(&mut cols[0], &mut cfg);
                        self.preview_section(&mut cols[1], &cfg);
                        self.light_section(&mut cols[2], &mut cfg);
                    });
                });
                if Some(&cfg) != self.edit.as_ref() {
                    self.edit = Some(cfg);
                }
            });
    }
}

impl HanaApp {
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .show_separator_line(true)
            .frame(egui::Frame::default().fill(BG).inner_margin(Margin::symmetric(28, 14)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(6.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(3), ACCENT);
                    ui.label(RichText::new("HANA CONTROL").size(20.0).strong());
                    ui.label(RichText::new("Redragon M694").size(15.0).color(MUTED));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // zębatka – ustawienia aplikacji
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(38.0), Sense::click());
                        let fill = if self.settings_open {
                            ACCENT
                        } else if resp.hovered() {
                            CARD_HI
                        } else {
                            CARD
                        };
                        ui.painter().rect_filled(r, CornerRadius::same(7), fill);
                        let icon_col = if self.settings_open { Color32::WHITE } else { TEXT };
                        crate::icons::gear(ui.painter(), egui::Rect::from_center_size(r.center(), Vec2::splat(20.0)), icon_col, fill);
                        let tip = if self.settings_open { "Wróć do ustawień myszy" } else { "Ustawienia aplikacji" };
                        if resp.on_hover_text(tip).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            self.settings_open = !self.settings_open;
                        }
                        ui.add_space(22.0);

                        // bateria: ikona z poziomem, piorun przy ładowaniu
                        if let Some(b) = self.battery {
                            let col = if b.percent <= 15 && b.charging == 0 { ACCENT } else { TEXT };
                            ui.label(RichText::new(format!("{}%", b.percent)).color(col).strong());
                            let (r, resp) = ui.allocate_exact_size(Vec2::new(30.0, 22.0), Sense::hover());
                            crate::icons::battery(ui.painter(), r, b.percent, b.charging != 0, TEXT, BG);
                            resp.on_hover_text(match b.charging {
                                1 => "Bateria – ładowanie",
                                2 => "Bateria – naładowana",
                                _ => "Bateria",
                            });
                            ui.add_space(22.0);
                        }

                        // połączenie: 2.4G (odbiornik) albo USB (kabel)
                        let (txt, col, tip) = if !self.connected() {
                            ("Brak połączenia", MUTED, "Mysz niepołączona")
                        } else if self.wired {
                            ("USB", GOOD, "Połączono kablem USB")
                        } else {
                            ("2.4G", GOOD, "Połączono przez odbiornik 2.4 GHz")
                        };
                        ui.label(RichText::new(txt).color(TEXT));
                        let (r, resp) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::hover());
                        if self.connected() && self.wired {
                            crate::icons::usb(ui.painter(), r, col);
                        } else {
                            crate::icons::wireless(ui.painter(), r, col);
                        }
                        resp.on_hover_text(tip);
                    });
                });
            });
    }

    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("footer")
            .show_separator_line(true)
            .frame(egui::Frame::default().fill(BG).inner_margin(Margin::symmetric(28, 12)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.dirty() {
                        ui.label(RichText::new("Niezapisane zmiany").color(ACCENT).strong());
                        ui.add_space(8.0);
                    }
                    if self.busy {
                        ui.spinner();
                    }
                    ui.label(RichText::new(&self.status).color(MUTED));

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can = self.connected() && !self.busy;
                        let apply = ui.add_enabled(
                            can && self.dirty(),
                            egui::Button::new(RichText::new("Zastosuj").color(Color32::WHITE).strong())
                                .fill(ACCENT)
                                .corner_radius(CornerRadius::same(7))
                                .min_size(Vec2::new(130.0, 38.0)),
                        );
                        if apply.clicked() {
                            if let Some(cfg) = self.edit.clone() {
                                self.send(Request::Write(cfg), "Zapisywanie…");
                            }
                        }
                        let btn = |t: &str| {
                            egui::Button::new(t).fill(CARD).corner_radius(CornerRadius::same(7)).min_size(Vec2::new(0.0, 38.0))
                        };
                        if ui.add_enabled(can && self.dirty(), btn("Cofnij zmiany")).clicked() {
                            self.edit = self.base.clone();
                            if let Some(b) = &self.base {
                                self.stage = b.active_stage();
                            }
                            self.status = "Cofnięto niezapisane zmiany.".into();
                        }
                        if ui.add_enabled(!self.busy, btn("Odczytaj z myszy")).clicked() {
                            self.send(Request::Reload, "Odczytywanie…");
                        }
                        let factory = ui
                            .add_enabled(can, btn("Domyślne producenta"))
                            .on_hover_text("DPI 800/1200/1600/3200/7200 i niebieskie podświetlenie wg instrukcji.\nZapisz przyciskiem Zastosuj.");
                        if factory.clicked() {
                            if let Some(cfg) = self.edit.as_mut() {
                                cfg.apply_factory();
                                self.stage = 0;
                                self.last_effect = 0;
                                self.status = "Wczytano ustawienia domyślne – kliknij Zastosuj.".into();
                            }
                        }
                    });
                });
            });
    }

    fn no_device(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            if self.busy {
                ui.spinner();
                ui.label(RichText::new("Łączenie z myszą…").size(18.0));
            } else {
                ui.label(RichText::new("Nie znaleziono myszy").size(22.0).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(&self.status).color(MUTED));
                ui.label(RichText::new("Aplikacja sama połączy się, gdy mysz się pojawi.").color(MUTED));
                ui.add_space(12.0);
                if choice(ui, false, "Szukaj ponownie", 160.0) {
                    self.send(Request::Reload, "Szukanie myszy…");
                }
            }
        });
    }

    /// Wiersz ustawienia: tytuł, opis i przełącznik po prawej.
    fn setting_row(ui: &mut egui::Ui, title: &str, desc: &str, on: bool) -> bool {
        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(15.0));
                ui.label(RichText::new(desc).size(13.0).color(MUTED));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                clicked = toggle(ui, on);
            });
        });
        ui.add_space(10.0);
        clicked
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let full = ui.available_width();
        let w = full.min(680.0);
        ui.horizontal(|ui| {
            ui.add_space((full - w) / 2.0);
            ui.vertical(|ui| {
                ui.set_width(w);
                ui.add_space(6.0);
                ui.label(RichText::new("Ustawienia aplikacji").size(22.0).strong());
                ui.add_space(22.0);

                section(ui, "URUCHAMIANIE", |ui| {
                    if Self::setting_row(
                        ui,
                        "Uruchamiaj razem z Windows",
                        "Po zalogowaniu aplikacja startuje schowana w zasobniku obok zegara.",
                        self.autostart,
                    ) {
                        match crate::win::set_autostart(!self.autostart) {
                            Ok(()) => self.autostart = !self.autostart,
                            Err(e) => self.status = format!("Nie udało się zmienić autostartu: {e}"),
                        }
                    }
                    ui.label(
                        RichText::new("Zamknięcie okna (X) chowa aplikację do zasobnika. Aby ją zakończyć, kliknij ikonę prawym przyciskiem i wybierz Zamknij aplikację.")
                            .size(13.0)
                            .color(MUTED),
                    );
                });

                section(ui, "POWIADOMIENIA", |ui| {
                    let notify = crate::win::low_battery_notify();
                    if Self::setting_row(
                        ui,
                        &format!("Ostrzeżenie o niskiej baterii ({}%)", crate::win::LOW_BATTERY),
                        "Powiadomienie Windows, gdy bateria myszy spadnie do 10% i mysz się nie ładuje.",
                        notify,
                    ) {
                        crate::win::set_low_battery_notify(!notify);
                    }
                    ui.horizontal(|ui| {
                        if choice(ui, false, "Wyślij testowe powiadomienie", 0.0) {
                            let res = crate::win::try_notify(
                                "Niski poziom baterii myszy",
                                "To jest test – tak będzie wyglądać ostrzeżenie przy 10% baterii.",
                            );
                            self.notify_result = Some(match res {
                                Err(e) => (e, false),
                                Ok(()) => match crate::win::notifications_paused_reason() {
                                    Some(why) => (
                                        format!("Wysłano, ale Windows może go nie pokazać: {why}. Sprawdź centrum powiadomień (Win+N)."),
                                        false,
                                    ),
                                    None => ("Wysłano – powiadomienie powinno pojawić się w prawym dolnym rogu.".into(), true),
                                },
                            });
                        }
                        if let Some((msg, ok)) = &self.notify_result {
                            ui.label(RichText::new(msg).size(13.0).color(if *ok { GOOD } else { ACCENT }));
                        }
                    });
                    ui.label(
                        RichText::new("Nie widzisz powiadomień? Wyłącz tryb „Nie przeszkadzać” w Windows (Win+N) i sprawdź, czy „Hana Control” jest włączona w Ustawieniach Windows > System > Powiadomienia.")
                            .size(12.5)
                            .color(MUTED),
                    );
                });

                section(ui, "KOPIA USTAWIEŃ MYSZY", |ui| {
                    ui.label(
                        RichText::new("Ustawienia myszy zapisane przy pierwszym połączeniu z aplikacją.")
                            .size(13.0)
                            .color(MUTED),
                    );
                    ui.add_space(4.0);
                    let can = self.connected() && !self.busy && load_backup().is_some();
                    ui.add_enabled_ui(can, |ui| {
                        if choice(ui, false, "Przywróć oryginalne ustawienia myszy", 0.0) {
                            if let Some(cfg) = load_backup() {
                                self.send(Request::Write(cfg), "Przywracanie kopii…");
                                self.settings_open = false;
                            }
                        }
                    });
                });
            });
        });
    }

    fn dpi_section(&mut self, ui: &mut egui::Ui, cfg: &mut Config) {
        section(ui, "CZUŁOŚĆ (DPI)", |ui| {
            // wybór poziomu
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let w = ((ui.available_width() - 6.0 * 4.0) / 5.0).floor().max(60.0);
                for i in 0..STAGES {
                    // kliknięcie wybiera poziom jako aktywny – trafia do myszy po „Zastosuj”
                    if self.stage_button(ui, cfg, i, w) {
                        self.stage = i;
                        cfg.set_stage_enabled(i, true);
                        cfg.set_active_stage(i);
                    }
                }
            });

            let i = self.stage;
            sublabel(ui, &format!("Poziom {} – wartość", i + 1));
            ui.horizontal_wrapped(|ui| {
                let cur = cfg.stage_dpi(i);
                let w = ((ui.available_width() - 8.0 * 3.0) / 4.0).floor().max(60.0);
                for dpi in DPI_VALUES {
                    if choice(ui, cur == dpi, &dpi.to_string(), w) {
                        cfg.set_stage_dpi(i, dpi);
                    }
                }
            });
        });
    }

    /// Przycisk poziomu DPI: numer i wartość; kropka = poziom aktywny w myszy.
    fn stage_button(&self, ui: &mut egui::Ui, cfg: &Config, i: usize, w: f32) -> bool {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, 58.0), Sense::click());
        let p = ui.painter();
        let selected = self.stage == i;
        let enabled = cfg.stage_enabled(i);
        let fill = if selected { ACCENT } else if resp.hovered() { CARD_HI } else { CARD };
        p.rect_filled(rect, CornerRadius::same(7), fill);
        let (top, main) = if selected {
            (Color32::from_rgb(0xFF, 0xD8, 0xDC), Color32::WHITE)
        } else {
            (MUTED, if enabled { TEXT } else { MUTED })
        };
        p.text(rect.min + Vec2::new(10.0, 8.0), egui::Align2::LEFT_TOP, format!("Poziom {}", i + 1), FontId::proportional(12.0), top);
        p.text(
            rect.min + Vec2::new(10.0, 25.0),
            egui::Align2::LEFT_TOP,
            if enabled { cfg.stage_dpi(i).to_string() } else { "wył.".into() },
            FontId::proportional(19.0),
            main,
        );
        let active = cfg.active_stage() == i;
        if active {
            p.circle_filled(rect.right_top() + Vec2::new(-11.0, 12.0), 4.0, if selected { Color32::WHITE } else { GOOD });
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        let resp = if active { resp.on_hover_text("Aktywny poziom") } else { resp };
        resp.clicked()
    }

    fn preview_section(&mut self, ui: &mut egui::Ui, cfg: &Config) {
        let dpi_color = cfg.stage_color(cfg.active_stage());
        let h = ui.available_height().clamp(560.0, 660.0);
        self.preview.show(ui, cfg, dpi_color, h);
    }

    fn polling_section(&mut self, ui: &mut egui::Ui, cfg: &mut Config) {
        section(ui, "CZĘSTOTLIWOŚĆ ODŚWIEŻANIA", |ui| {
            ui.horizontal(|ui| {
                let w = ((ui.available_width() - 8.0 * 3.0) / 4.0).floor().max(60.0);
                for (idx, hz) in POLLING_HZ.iter().enumerate() {
                    if choice(ui, cfg.polling() == idx as u8, &format!("{hz} Hz"), w) {
                        cfg.set_polling(idx as u8);
                    }
                }
            });
        });
    }

    fn light_section(&mut self, ui: &mut egui::Ui, cfg: &mut Config) {
        let off = cfg.effect() >= EFFECT_OFF;
        section(ui, "PODŚWIETLENIE", |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(if off { "Podświetlenie wyłączone" } else { "Podświetlenie włączone" }).size(15.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if toggle(ui, !off) {
                        if off {
                            cfg.set_effect(self.last_effect);
                            if cfg.brightness() == 0 {
                                cfg.set_brightness(4);
                            }
                        } else {
                            self.last_effect = cfg.effect();
                            cfg.set_effect(EFFECT_OFF);
                        }
                    }
                });
            });

            ui.add_enabled_ui(!off, |ui| {
                sublabel(ui, "Efekt");
                let w = ((ui.available_width() - 8.0) / 2.0).floor().max(100.0);
                for row in EFFECTS.chunks(2) {
                    ui.horizontal(|ui| {
                        for (code, name) in row {
                            if choice(ui, cfg.effect() == *code, name, w) {
                                cfg.set_effect(*code);
                                self.last_effect = *code;
                            }
                        }
                    });
                }

                let uses_color = effect_uses_color(cfg.effect());
                sublabel(ui, if uses_color { "Kolor" } else { "Kolor – ten efekt jest wielokolorowy" });
                ui.add_enabled_ui(uses_color, |ui| {
                    let (picked, rainbow) = palette(ui, cfg.light_color(), self.wheel_open);
                    if let Some(c) = picked {
                        cfg.set_light_color(c);
                        self.wheel_open = false;
                    }
                    if rainbow {
                        self.wheel_open = !self.wheel_open;
                    }
                    if !self.wheel_open {
                        return;
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if let Some(c) = color_wheel(ui, cfg.light_color(), 168.0) {
                            cfg.set_light_color(c);
                        }
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            ui.add_space(44.0);
                            ui.label(RichText::new("Kolor własny").size(13.0).color(MUTED));
                            let cur = cfg.light_color();
                            let (r, _) = ui.allocate_exact_size(Vec2::new(64.0, 34.0), Sense::hover());
                            ui.painter().rect_filled(r, CornerRadius::same(7), rgb(cur));
                            ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", cur[0], cur[1], cur[2])).size(13.0));
                            ui.label(RichText::new("Kliknij lub przeciągnij\npo kole barw").size(12.0).color(MUTED));
                        });
                    });
                });

                sublabel(ui, "Jasność");
                ui.horizontal(|ui| {
                    let w = ((ui.available_width() - 8.0 * 3.0) / 4.0).floor().max(50.0);
                    for (n, lvl) in BRIGHTNESS_LEVELS.iter().enumerate() {
                        let label = ["25%", "50%", "75%", "100%"][n];
                        if choice(ui, cfg.brightness() == *lvl, label, w) {
                            cfg.set_brightness(*lvl);
                        }
                    }
                });

                let anim = cfg.effect() != 0;
                sublabel(ui, if anim { "Prędkość (1 = najwolniej, 5 = najszybciej)" } else { "Prędkość – nie dotyczy efektu Stały" });
                ui.add_enabled_ui(anim, |ui| {
                    ui.horizontal(|ui| {
                        let w = ((ui.available_width() - 8.0 * 4.0) / 5.0).floor().max(40.0);
                        let cur = cfg.speed_raw();
                        // najbliższy poziom do wartości z myszy
                        let cur_idx = SPEED_RAW
                            .iter()
                            .enumerate()
                            .min_by_key(|(_, r)| (**r as i16 - cur as i16).abs())
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        for (n, raw) in SPEED_RAW.iter().enumerate() {
                            if choice(ui, cur_idx == n, &(n + 1).to_string(), w) {
                                cfg.set_speed_raw(*raw);
                            }
                        }
                    });
                });
            });
        });
    }
}
