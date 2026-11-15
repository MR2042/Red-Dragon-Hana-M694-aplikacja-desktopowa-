#![windows_subsystem = "windows"] // bez okna konsoli

mod app;
mod config;
mod device;
mod icons;
mod preview;
mod tray;
mod win;
mod worker;

use std::sync::Arc;

fn main() -> eframe::Result {
    // Diagnostyka: HanaControl.exe --test-notify zapisuje wynik wysłania powiadomienia do %TEMP%\hana_notify.txt
    if std::env::args().any(|a| a == "--test-notify") {
        let res = win::try_notify("Hana Control – test", "Powiadomienie testowe z trybu diagnostycznego.");
        let paused = win::notifications_paused_reason();
        let _ = std::fs::write(std::env::temp_dir().join("hana_notify.txt"), format!("wynik: {res:?}\nwstrzymane: {paused:?}\n"));
        return Ok(());
    }
    // Druga kopia tylko pokazuje okno pierwszej i kończy działanie.
    if !win::ensure_single_instance() {
        return Ok(());
    }
    // "--tray" – start z autostartu: od razu schowane w zasobniku.
    let start_hidden = std::env::args().any(|a| a == "--tray");

    win::load_settings();
    win::register_notifications();
    win::refresh_autostart_path();

    let icon = eframe::egui::IconData {
        rgba: include_bytes!("../assets/icon64.rgba").to_vec(),
        width: 64,
        height: 64,
    };
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(win::WINDOW_TITLE)
            .with_inner_size([1500.0, 900.0])
            .with_min_inner_size([1250.0, 760.0])
            .with_icon(Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "Hana Control",
        options,
        Box::new(move |cc| Ok(Box::new(app::HanaApp::new(cc, start_hidden)))),
    )
}
