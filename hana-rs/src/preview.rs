//! Podgląd podświetlenia na zdjęciu myszy M694 (widok z góry).
//! Pasek LED przy podstawie widać z góry jako poświatę dookoła obrysu myszy,
//! a diody kółka świecą kolorem aktywnego poziomu DPI.

use std::f32::consts::TAU;

use eframe::egui::epaint::{Mesh, Vertex, WHITE_UV};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, TextureHandle, Vec2};

use crate::config::{Config, EFFECT_OFF, Rgb};

static TOP_PNG: &[u8] = include_bytes!("../assets/m694_top.png");
static WHEEL_PNG: &[u8] = include_bytes!("../assets/m694_wheel.png");

/// Liczba punktów obrysu myszy (do rysowania poświaty).
const OUTLINE_POINTS: usize = 220;

fn decode_png(bytes: &[u8]) -> egui::ColorImage {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().expect("PNG");
    let mut buf = vec![0; reader.output_buffer_size().expect("PNG")];
    let info = reader.next_frame(&mut buf).expect("PNG");
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => buf[..info.buffer_size()].chunks(3).flat_map(|c| [c[0], c[1], c[2], 255]).collect(),
        _ => vec![0; info.width as usize * info.height as usize * 4],
    };
    egui::ColorImage::from_rgba_unmultiplied([info.width as usize, info.height as usize], &rgba)
}

/// Obrys myszy: promienie od środka zdjęcia do krawędzi (w pikselach zdjęcia).
fn outline_from_alpha(img: &egui::ColorImage) -> Vec<Vec2> {
    let [w, h] = img.size;
    let c = Vec2::new(w as f32 / 2.0, h as f32 / 2.0);
    let alpha = |x: f32, y: f32| -> u8 {
        let (xi, yi) = (x as isize, y as isize);
        if xi < 0 || yi < 0 || xi >= w as isize || yi >= h as isize {
            return 0;
        }
        img.pixels[yi as usize * w + xi as usize].a()
    };
    let mut pts: Vec<Vec2> = (0..OUTLINE_POINTS)
        .map(|i| {
            let a = i as f32 / OUTLINE_POINTS as f32 * TAU;
            let dir = Vec2::new(a.sin(), -a.cos()); // 0 = przód (góra zdjęcia), dalej zgodnie z zegarem
            let mut r = (w.max(h)) as f32;
            while r > 0.0 {
                let p = c + dir * r;
                if alpha(p.x, p.y) > 110 {
                    break;
                }
                r -= 1.0;
            }
            c + dir * r
        })
        .collect();
    // wygładzenie, żeby poświata nie łapała drobnych nierówności tekstury
    for _ in 0..3 {
        let prev = pts.clone();
        let n = pts.len();
        for i in 0..n {
            pts[i] = (prev[(i + n - 1) % n] + prev[i] * 2.0 + prev[(i + 1) % n]) / 4.0;
        }
    }
    pts
}

// ---------- efekty świetlne ----------
fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(1.0) * 6.0;
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    match i {
        0 => [v, t, p],
        1 => [q, v, p],
        2 => [p, v, t],
        3 => [p, q, v],
        4 => [t, p, v],
        _ => [v, p, q],
    }
}
fn scale(c: [f32; 3], s: f32) -> [f32; 3] {
    [c[0] * s, c[1] * s, c[2] * s]
}
fn breath(f: f32) -> f32 {
    (0.5 - 0.5 * (TAU * f).cos()).powf(1.3)
}
fn rgbf(c: Rgb) -> [f32; 3] {
    [c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0]
}

/// Kolor paska LED w danym miejscu i chwili – symulacja efektów z instrukcji.
/// `along`: 0 przód … 1 tył; `around`: 0–1 dookoła myszy.
fn light_at(cfg: &Config, along: f32, around: f32, t: f32) -> [f32; 3] {
    let code = cfg.effect();
    if code >= EFFECT_OFF {
        return [0.0; 3];
    }
    let bright = cfg.brightness().min(4) as f32 / 4.0;
    let period = 1.2 + 3.8 * (cfg.speed_raw().min(100) as f32 / 82.0);
    let f = t / period;
    let base = rgbf(cfg.light_color());
    let c = match code {
        0 => base,
        1 => scale(base, breath(f)),
        2 => {
            // kolorowy ogon: impuls biegnący od przodu do tyłu, za każdym razem w innym kolorze
            let ph = f - along * 0.6;
            let k = ph.floor();
            let x = ph - k;
            let i = if x < 0.2 { x / 0.2 } else { (1.0 - (x - 0.2) / 0.8).powi(2) };
            scale(hsv(k * 0.18, 1.0, 1.0), i)
        }
        3 => hsv(f * 0.25, 1.0, 1.0),
        4 => hsv(around - f * 0.5, 1.0, 1.0),
        5 => scale(hsv(f.floor() * 0.16, 1.0, 1.0), breath(f)),
        _ => [0.0; 3],
    };
    scale(c, bright)
}

/// Kolor dodawany (alfa 0 w egui = mieszanie addytywne) – daje efekt świecenia.
fn additive(c: [f32; 3], k: f32) -> Color32 {
    let g = |x: f32| ((x * k).clamp(0.0, 1.0) * 255.0) as u8;
    Color32::from_rgba_premultiplied(g(c[0]), g(c[1]), g(c[2]), 0)
}

pub struct Preview {
    top: TextureHandle,
    wheel: TextureHandle,
    img_size: Vec2,
    outline: Vec<Vec2>,
}

impl Preview {
    pub fn new(ctx: &egui::Context) -> Self {
        let top_img = decode_png(TOP_PNG);
        let wheel_img = decode_png(WHEEL_PNG);
        let img_size = Vec2::new(top_img.size[0] as f32, top_img.size[1] as f32);
        let outline = outline_from_alpha(&top_img);
        let opts = egui::TextureOptions::LINEAR;
        Self {
            top: ctx.load_texture("m694_top", top_img, opts),
            wheel: ctx.load_texture("m694_wheel", wheel_img, opts),
            img_size,
            outline,
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, cfg: &Config, dpi_color: Rgb, height: f32) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
        let t = ui.input(|i| i.time) as f32;
        let painter = ui.painter_at(rect);

        // dopasowanie zdjęcia z marginesem na poświatę
        let pad = 46.0;
        let s = ((rect.width() - 2.0 * pad) / self.img_size.x).min((rect.height() - 2.0 * pad) / self.img_size.y);
        let img_rect = Rect::from_center_size(rect.center(), self.img_size * s);
        let to_screen = |p: Vec2| img_rect.min + p * s;

        // poświata paska LED dookoła obrysu
        if cfg.effect() < EFFECT_OFF {
            let n = self.outline.len();
            let mut mesh = Mesh::default();
            // pierścienie: (odsunięcie od krawędzi w px, siła)
            // łagodny zanik (rozmycie): więcej warstw o malejącej sile
            let rings = [(-6.0f32, 0.55f32), (0.0, 0.7), (4.0, 0.72), (9.0, 0.6), (15.0, 0.44), (23.0, 0.28), (33.0, 0.15), (46.0, 0.06), (62.0, 0.015), (78.0, 0.0)];
            for i in 0..n {
                let p = self.outline[i];
                let prev = self.outline[(i + n - 1) % n];
                let next = self.outline[(i + 1) % n];
                let tangent = (next - prev).normalized();
                let normal = Vec2::new(-tangent.y, tangent.x); // obrys idzie zgodnie z zegarem → normalna na zewnątrz
                let along = (p.y / self.img_size.y).clamp(0.0, 1.0);
                let around = i as f32 / n as f32;
                // pasek LED nie obejmuje przodu myszy (przyciski) – poświata zaczyna się za nim, z łagodnym przejściem
                let x = ((along - 0.15) / 0.07).clamp(0.0, 1.0);
                let front_fade = x * x * (3.0 - 2.0 * x);
                let c = light_at(cfg, along, around, t);
                let sp = to_screen(p);
                for (off, k) in rings {
                    mesh.vertices.push(Vertex {
                        pos: sp + normal * off,
                        uv: WHITE_UV,
                        color: additive(c, k * 1.3 * front_fade),
                    });
                }
            }
            let r = rings.len() as u32;
            for i in 0..n as u32 {
                let j = (i + 1) % n as u32;
                for k in 0..r - 1 {
                    let (a, b, c, d) = (i * r + k, j * r + k, j * r + k + 1, i * r + k + 1);
                    mesh.indices.extend_from_slice(&[a, b, c, a, c, d]);
                }
            }
            painter.add(egui::Shape::mesh(mesh));
        }

        // zdjęcie myszy
        let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        painter.image(self.top.id(), img_rect, uv, Color32::WHITE);

        // diody kółka w kolorze aktywnego poziomu DPI (z delikatnym pulsowaniem jak w myszy)
        let k = 0.55 + 0.45 * breath(t / 3.0);
        let c = rgbf(dpi_color);
        let tint = Color32::from_rgb((c[0] * 255.0 * k) as u8, (c[1] * 255.0 * k) as u8, (c[2] * 255.0 * k) as u8);
        painter.image(self.wheel.id(), img_rect, uv, tint);
        painter.image(self.wheel.id(), img_rect, uv, additive(c, 0.6 * k)); // dodatkowy blask

        if cfg.effect() >= EFFECT_OFF {
            painter.text(
                rect.center_bottom() + Vec2::new(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                "Podświetlenie wyłączone",
                egui::FontId::proportional(13.0),
                Color32::from_rgb(0x8C, 0x93, 0xA1),
            );
        }

        // płynność dopasowana do sytuacji – mniej klatek = mniejsze zużycie procesora
        let focused = ui.input(|i| i.viewport().focused.unwrap_or(true));
        let ms = if !focused { 200 } else if matches!(cfg.effect(), 1..=5) || resp.hovered() { 33 } else { 100 };
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(ms));
    }
}
