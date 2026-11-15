//! Ikony rysowane wektorowo (bez czcionek – zawsze wyglądają tak samo).

use std::f32::consts::{PI, TAU};

use eframe::egui::{self, Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, Vec2, epaint::Mesh};

fn arc_points(c: Pos2, r: f32, a0: f32, a1: f32, n: usize) -> Vec<Pos2> {
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        c + Vec2::new(a.cos(), a.sin()) * r
    }).collect()
}

/// Łączność bezprzewodowa 2.4G: kropka i trzy łuki sygnału.
pub fn wireless(p: &Painter, rect: Rect, color: Color32) {
    let c = Pos2::new(rect.center().x, rect.bottom() - 2.5);
    let s = rect.height() / 18.0;
    p.circle_filled(c, 2.0 * s, color);
    for (k, r) in [5.5f32, 10.0, 14.5].iter().enumerate() {
        let w = 1.9 * s;
        let pts = arc_points(c, r * s, -PI * 0.5 - 0.78, -PI * 0.5 + 0.78, 14);
        let _ = k;
        p.add(Shape::line(pts, Stroke::new(w, color)));
    }
}

/// Symbol USB (trójząb).
pub fn usb(p: &Painter, rect: Rect, color: Color32) {
    let s = rect.height() / 18.0;
    let c = rect.center();
    let top = Pos2::new(c.x, rect.top() + 1.0);
    let bottom = Pos2::new(c.x, rect.bottom() - 3.0 * s);
    let st = Stroke::new(1.7 * s, color);
    p.line_segment([top + Vec2::new(0.0, 3.0 * s), bottom], st);
    // grot na górze
    p.add(Shape::convex_polygon(
        vec![top, top + Vec2::new(-2.8 * s, 4.2 * s), top + Vec2::new(2.8 * s, 4.2 * s)],
        color,
        Stroke::NONE,
    ));
    // kółko na dole
    p.circle_filled(bottom, 2.4 * s, color);
    // lewe ramię zakończone kółkiem
    let l0 = Pos2::new(c.x, c.y + 3.0 * s);
    let l1 = Pos2::new(c.x - 5.0 * s, c.y - 0.5 * s);
    let l2 = Pos2::new(c.x - 5.0 * s, c.y - 3.5 * s);
    p.add(Shape::line(vec![l0, l1, l2], st));
    p.circle_filled(l2, 1.8 * s, color);
    // prawe ramię zakończone kwadratem
    let r0 = Pos2::new(c.x, c.y + 0.5 * s);
    let r1 = Pos2::new(c.x + 5.0 * s, c.y - 2.5 * s);
    let r2 = Pos2::new(c.x + 5.0 * s, c.y - 5.0 * s);
    p.add(Shape::line(vec![r0, r1, r2], st));
    p.rect_filled(Rect::from_center_size(r2, Vec2::splat(3.4 * s)), CornerRadius::ZERO, color);
}

/// Bateria z poziomem naładowania; `charging` dodaje piorun.
pub fn battery(p: &Painter, rect: Rect, percent: u8, charging: bool, outline: Color32, bg: Color32) {
    let h = rect.height() * 0.62;
    let body = Rect::from_min_size(Pos2::new(rect.left(), rect.center().y - h / 2.0), Vec2::new(rect.width() - 3.0, h));
    let nub = Rect::from_min_size(Pos2::new(body.right(), body.center().y - h * 0.22), Vec2::new(2.5, h * 0.44));
    p.rect_stroke(body, CornerRadius::same(3), Stroke::new(1.6_f32, outline), egui::StrokeKind::Inside);
    p.rect_filled(nub, CornerRadius::same(1), outline);
    let fill_col = if charging || percent > 30 {
        Color32::from_rgb(0x2E, 0xCC, 0x71)
    } else if percent > 15 {
        Color32::from_rgb(0xF3, 0x9C, 0x12)
    } else {
        Color32::from_rgb(0xE0, 0x2F, 0x44)
    };
    let inner = body.shrink(3.0);
    let w = inner.width() * (percent.min(100) as f32 / 100.0);
    if w > 0.5 {
        p.rect_filled(Rect::from_min_size(inner.min, Vec2::new(w, inner.height())), CornerRadius::same(1), fill_col);
    }
    if charging {
        bolt(p, body.center(), h * 0.95, Color32::WHITE, bg);
    }
}

/// Piorun (ładowanie) z obwódką w kolorze tła, żeby był czytelny na pasku baterii.
fn bolt(p: &Painter, c: Pos2, h: f32, color: Color32, outline: Color32) {
    let s = h / 14.0;
    let pt = |x: f32, y: f32| c + Vec2::new((x - 5.0) * s, (y - 7.0) * s);
    let poly = [pt(6.5, 0.0), pt(1.0, 8.0), pt(5.0, 8.0), pt(3.5, 14.0), pt(9.0, 5.5), pt(5.0, 5.5)];
    p.add(Shape::closed_line(poly.to_vec(), Stroke::new(2.2_f32, outline)));
    let mut mesh = Mesh::default();
    for q in poly {
        mesh.colored_vertex(q, color);
    }
    // dwa trójkąty górnej i dolnej części pioruna + łącznik
    for [a, b, cc] in [[0u32, 1, 2], [0, 2, 5], [5, 2, 4], [2, 3, 4]] {
        mesh.add_triangle(a, b, cc);
    }
    p.add(Shape::mesh(mesh));
}

/// Zębatka ustawień.
pub fn gear(p: &Painter, rect: Rect, color: Color32, hole: Color32) {
    let c = rect.center();
    let r = rect.height().min(rect.width()) / 2.0;
    let teeth = 8;
    for k in 0..teeth {
        let a = k as f32 / teeth as f32 * TAU;
        let dir = Vec2::new(a.cos(), a.sin());
        let side = Vec2::new(-dir.y, dir.x);
        let (r0, r1, w) = (r * 0.55, r * 0.98, r * 0.22);
        p.add(Shape::convex_polygon(
            vec![c + dir * r0 - side * w, c + dir * r1 - side * w * 0.8, c + dir * r1 + side * w * 0.8, c + dir * r0 + side * w],
            color,
            Stroke::NONE,
        ));
    }
    p.circle_filled(c, r * 0.72, color);
    p.circle_filled(c, r * 0.3, hole);
}

/// Tęczowe kółko (otwiera koło barw).
pub fn rainbow_circle(p: &Painter, c: Pos2, r: f32) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(c, Color32::WHITE);
    let n = 36;
    for i in 0..=n {
        let h = i as f32 / n as f32;
        let a = h * TAU;
        let [cr, cg, cb] = crate::app::hsv_to_rgb(h, 1.0, 1.0);
        mesh.colored_vertex(c + Vec2::new(a.cos(), a.sin()) * r, Color32::from_rgb(cr, cg, cb));
    }
    for i in 1..=n as u32 {
        mesh.add_triangle(0, i, i + 1);
    }
    p.add(Shape::mesh(mesh));
}
