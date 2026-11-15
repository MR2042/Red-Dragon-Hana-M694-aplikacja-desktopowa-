#![windows_subsystem = "windows"] // bez okna konsoli

//! Instalator Hana Control.
//! - instaluje dla bieżącego użytkownika (bez uprawnień administratora) do %LOCALAPPDATA%\Programs\Hana Control,
//! - tworzy skróty, opcjonalnie autostart, wpis w „Aplikacje zainstalowane”,
//! - uruchomiony z `--uninstall` działa jako deinstalator.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use eframe::egui::{self, Align, Color32, CornerRadius, FontId, Layout, Margin, RichText, Sense, Stroke, Vec2};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

static APP_EXE: &[u8] = include_bytes!("../../hana-rs/target/release/hana_control.exe");
const VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "Hana Control";
const EXE_NAME: &str = "HanaControl.exe";
const UNINSTALL_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Uninstall\HanaControl";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "HanaControl";
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// kolory jak w aplikacji
const BG: Color32 = Color32::from_rgb(0x13, 0x15, 0x19);
const CARD: Color32 = Color32::from_rgb(0x22, 0x26, 0x2E);
const CARD_HI: Color32 = Color32::from_rgb(0x2C, 0x31, 0x3B);
const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xEE);
const MUTED: Color32 = Color32::from_rgb(0x8C, 0x93, 0xA1);
const ACCENT: Color32 = Color32::from_rgb(0xE0, 0x2F, 0x44);
const GOOD: Color32 = Color32::from_rgb(0x2E, 0xCC, 0x71);

// ---------- ścieżki ----------
fn env_dir(var: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(var).unwrap_or_default())
}
fn install_dir() -> PathBuf {
    env_dir("LOCALAPPDATA").join("Programs").join(APP_NAME)
}
fn start_menu_link() -> PathBuf {
    env_dir("APPDATA").join(r"Microsoft\Windows\Start Menu\Programs").join(format!("{APP_NAME}.lnk"))
}
fn desktop_link() -> PathBuf {
    env_dir("USERPROFILE").join("Desktop").join(format!("{APP_NAME}.lnk"))
}
fn data_dir() -> PathBuf {
    env_dir("APPDATA").join("HanaControl")
}
fn installed_version() -> Option<String> {
    RegKey::predef(HKEY_CURRENT_USER).open_subkey(UNINSTALL_KEY).and_then(|k| k.get_value("DisplayVersion")).ok()
}

fn hidden(cmd: &mut Command) -> &mut Command {
    cmd.creation_flags(CREATE_NO_WINDOW)
}

/// Zamyka działającą aplikację (np. przy aktualizacji lub odinstalowaniu).
fn close_running_app() {
    let _ = hidden(Command::new("taskkill").args(["/F", "/IM", EXE_NAME])).status();
    std::thread::sleep(std::time::Duration::from_millis(600));
}

/// Skrót .lnk przez WScript.Shell (wbudowany w Windows).
fn create_shortcut(link: &Path, target: &Path) -> Result<(), String> {
    let q = |p: &Path| p.display().to_string().replace('\'', "''");
    let script = format!(
        "$s=(New-Object -ComObject WScript.Shell).CreateShortcut('{l}');$s.TargetPath='{t}';$s.WorkingDirectory='{d}';$s.IconLocation='{t},0';$s.Description='Panel ustawień myszy Redragon M694';$s.Save()",
        l = q(link),
        t = q(target),
        d = q(target.parent().unwrap_or(Path::new("."))),
    );
    let ok = hidden(Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]))
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok { Ok(()) } else { Err(format!("Nie udało się utworzyć skrótu {}", link.display())) }
}

struct Options {
    desktop: bool,
    autostart: bool,
    launch: bool,
}

fn install(opt: &Options) -> Result<(), String> {
    let dir = install_dir();
    let exe = dir.join(EXE_NAME);
    let uninstaller = dir.join("uninstall.exe");
    close_running_app();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Nie można utworzyć folderu {}: {e}", dir.display()))?;
    std::fs::write(&exe, APP_EXE).map_err(|e| format!("Nie można zapisać {}: {e}", exe.display()))?;
    // kopia instalatora służy jako deinstalator
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    if me != uninstaller {
        std::fs::copy(&me, &uninstaller).map_err(|e| format!("Nie można zapisać deinstalatora: {e}"))?;
    }

    create_shortcut(&start_menu_link(), &exe)?;
    if opt.desktop {
        create_shortcut(&desktop_link(), &exe)?;
    } else {
        let _ = std::fs::remove_file(desktop_link());
    }

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if opt.autostart {
        run.set_value(RUN_VALUE, &format!("\"{}\" --tray", exe.display())).map_err(|e| e.to_string())?;
    } else {
        let _ = run.delete_value(RUN_VALUE);
    }

    // wpis w „Aplikacje zainstalowane”
    let (k, _) = hkcu.create_subkey(UNINSTALL_KEY).map_err(|e| e.to_string())?;
    let size_kb = ((APP_EXE.len() + std::fs::metadata(&me).map(|m| m.len() as usize).unwrap_or(0)) / 1024) as u32;
    let set = |name: &str, v: &str| k.set_value(name, &v).map_err(|e| e.to_string());
    set("DisplayName", APP_NAME)?;
    set("DisplayVersion", VERSION)?;
    set("Publisher", "Hana Control (projekt społecznościowy)")?;
    set("DisplayIcon", &format!("{},0", exe.display()))?;
    set("InstallLocation", &dir.display().to_string())?;
    set("UninstallString", &format!("\"{}\" --uninstall", uninstaller.display()))?;
    set("URLInfoAbout", "https://github.com/MR2042/Red-Dragon-Hana-M694-aplikacja-desktopowa-")?;
    k.set_value("EstimatedSize", &size_kb).map_err(|e| e.to_string())?;
    k.set_value("NoModify", &1u32).map_err(|e| e.to_string())?;
    k.set_value("NoRepair", &1u32).map_err(|e| e.to_string())?;

    if opt.launch {
        let _ = Command::new(&exe).current_dir(&dir).spawn();
    }
    Ok(())
}

fn uninstall(remove_data: bool) -> Result<(), String> {
    close_running_app();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    if let Ok(run) = hkcu.open_subkey_with_flags(RUN_KEY, winreg::enums::KEY_WRITE) {
        let _ = run.delete_value(RUN_VALUE);
    }
    let _ = std::fs::remove_file(start_menu_link());
    let _ = std::fs::remove_file(desktop_link());
    let _ = hkcu.delete_subkey_all(UNINSTALL_KEY);
    let _ = hkcu.delete_subkey_all(r"Software\Classes\AppUserModelId\Redragon.HanaControl");
    if remove_data {
        let _ = hkcu.delete_subkey_all(r"Software\HanaControl");
        let _ = std::fs::remove_dir_all(data_dir());
    }
    // folder instalacji usuwamy po zamknięciu deinstalatora (sam siebie nie może skasować)
    let dir = install_dir();
    let _ = std::fs::remove_file(dir.join(EXE_NAME));
    // czekamy na zamknięcie deinstalatora i ponawiamy próbę; katalog roboczy poza usuwanym folderem
    let d = dir.display();
    let cmd = format!(
        "ping 127.0.0.1 -n 3 > nul & rmdir /s /q \"{d}\" & if exist \"{d}\" (ping 127.0.0.1 -n 4 > nul & rmdir /s /q \"{d}\")"
    );
    // proces sprzątający musi przeżyć zamknięcie deinstalatora: odłączony i (jeśli można) poza grupą procesów
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
    // raw_arg: cmd.exe nie rozumie cudzysłowów poprzedzonych \, które dodaje zwykłe .arg()
    let spawn = |flags: u32| {
        Command::new("cmd")
            .raw_arg(format!("/C \"{cmd}\""))
            .current_dir(std::env::temp_dir())
            .creation_flags(flags)
            .spawn()
    };
    if spawn(CREATE_NO_WINDOW | DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB).is_err() {
        let _ = spawn(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }
    Ok(())
}

// ---------- interfejs ----------
#[derive(PartialEq)]
enum Page {
    Welcome,
    Done,
    Error(String),
}

struct SetupApp {
    uninstall_mode: bool,
    page: Page,
    opt: Options,
    remove_data: bool,
    previous: Option<String>,
}

fn choice_toggle(ui: &mut egui::Ui, on: bool, title: &str, desc: &str) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(15.0));
            ui.label(RichText::new(desc).size(12.5).color(MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (rect, resp) = ui.allocate_exact_size(Vec2::new(46.0, 24.0), Sense::click());
            let t = ui.ctx().animate_bool(resp.id, on);
            ui.painter().rect_filled(rect, CornerRadius::same(12), if on { ACCENT } else { CARD_HI });
            let x = egui::lerp(rect.left() + 12.0..=rect.right() - 12.0, t);
            ui.painter().circle_filled(egui::pos2(x, rect.center().y), 9.0, Color32::WHITE);
            clicked = resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked();
        });
    });
    ui.add_space(8.0);
    clicked
}

fn button(ui: &mut egui::Ui, text: &str, primary: bool) -> bool {
    let (fill, col) = if primary { (ACCENT, Color32::WHITE) } else { (CARD, TEXT) };
    ui.add(
        egui::Button::new(RichText::new(text).color(col).strong())
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(CornerRadius::same(7))
            .min_size(Vec2::new(130.0, 38.0)),
    )
    .clicked()
}

impl eframe::App for SetupApp {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(BG).inner_margin(Margin::symmetric(32, 26)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(Vec2::new(6.0, 26.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::same(3), ACCENT);
                    ui.label(RichText::new("HANA CONTROL").size(20.0).strong());
                    ui.label(RichText::new(format!("v{VERSION}")).size(14.0).color(MUTED));
                });
                ui.add_space(18.0);

                match &self.page {
                    Page::Welcome if self.uninstall_mode => {
                        ui.label(RichText::new("Odinstalowanie").size(22.0).strong());
                        ui.add_space(6.0);
                        ui.label(RichText::new("Usunie aplikację, skróty, autostart i wpis w „Aplikacje zainstalowane”.").color(MUTED));
                        ui.add_space(18.0);
                        if choice_toggle(
                            ui,
                            self.remove_data,
                            "Usuń także ustawienia aplikacji",
                            "Kopia oryginalnych ustawień myszy i opcje powiadomień (%APPDATA%\\HanaControl).",
                        ) {
                            self.remove_data = !self.remove_data;
                        }
                        ui.label(RichText::new("Ustawienia zapisane w pamięci myszy zostają bez zmian.").size(12.5).color(MUTED));
                    }
                    Page::Welcome => {
                        let title = match &self.previous {
                            Some(v) => format!("Aktualizacja (zainstalowana wersja {v})"),
                            None => "Instalacja".to_string(),
                        };
                        ui.label(RichText::new(title).size(22.0).strong());
                        ui.add_space(6.0);
                        ui.label(RichText::new("Panel ustawień myszy Redragon M694 Hana: DPI, częstotliwość i podświetlenie.").color(MUTED));
                        ui.label(
                            RichText::new(format!("Folder: {}", install_dir().display()))
                                .size(12.5)
                                .color(MUTED),
                        );
                        ui.add_space(18.0);
                        if choice_toggle(ui, self.opt.desktop, "Skrót na pulpicie", "Skrót w menu Start jest tworzony zawsze.") {
                            self.opt.desktop = !self.opt.desktop;
                        }
                        if choice_toggle(
                            ui,
                            self.opt.autostart,
                            "Uruchamiaj razem z Windows",
                            "Aplikacja startuje schowana w zasobniku i pilnuje baterii.",
                        ) {
                            self.opt.autostart = !self.opt.autostart;
                        }
                        if choice_toggle(ui, self.opt.launch, "Uruchom po instalacji", "") {
                            self.opt.launch = !self.opt.launch;
                        }
                    }
                    Page::Done => {
                        ui.add_space(30.0);
                        ui.vertical_centered(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(56.0), Sense::hover());
                            ui.painter().circle_filled(r.center(), 28.0, GOOD);
                            let c = r.center();
                            ui.painter().line_segment([c + Vec2::new(-12.0, 0.0), c + Vec2::new(-3.0, 9.0)], Stroke::new(4.0_f32, Color32::WHITE));
                            ui.painter().line_segment([c + Vec2::new(-3.0, 9.0), c + Vec2::new(13.0, -9.0)], Stroke::new(4.0_f32, Color32::WHITE));
                            ui.add_space(14.0);
                            let msg = if self.uninstall_mode { "Hana Control została odinstalowana." } else { "Hana Control jest zainstalowana." };
                            ui.label(RichText::new(msg).size(20.0).strong());
                            if !self.uninstall_mode {
                                ui.label(RichText::new("Znajdziesz ją w menu Start.").color(MUTED));
                            }
                        });
                    }
                    Page::Error(e) => {
                        ui.label(RichText::new("Wystąpił błąd").size(22.0).strong().color(ACCENT));
                        ui.add_space(8.0);
                        ui.label(RichText::new(e).color(TEXT));
                    }
                }

                // przyciski na dole
                ui.with_layout(Layout::bottom_up(Align::Max), |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| match self.page {
                            Page::Welcome => {
                                let label = if self.uninstall_mode {
                                    "Odinstaluj"
                                } else if self.previous.is_some() {
                                    "Aktualizuj"
                                } else {
                                    "Zainstaluj"
                                };
                                if button(ui, label, true) {
                                    let res = if self.uninstall_mode { uninstall(self.remove_data) } else { install(&self.opt) };
                                    self.page = match res {
                                        Ok(()) => Page::Done,
                                        Err(e) => Page::Error(e),
                                    };
                                }
                                if button(ui, "Anuluj", false) {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                            _ => {
                                if button(ui, "Zakończ", true) {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            }
                        });
                    });
                });
            });
    }
}

fn main() -> eframe::Result {
    let uninstall_mode = std::env::args().any(|a| a == "--uninstall");
    // tryb cichy (bez okna): instalacja z ustawieniami domyślnymi albo odinstalowanie
    if std::env::args().any(|a| a == "--silent") {
        let res = if uninstall_mode {
            uninstall(false)
        } else {
            install(&Options { desktop: false, autostart: false, launch: false })
        };
        std::process::exit(if res.is_ok() { 0 } else { 1 });
    }
    let icon = egui::IconData {
        rgba: include_bytes!("../../hana-rs/assets/icon64.rgba").to_vec(),
        width: 64,
        height: 64,
    };
    let title = if uninstall_mode { "Hana Control – odinstalowanie" } else { "Hana Control – instalator" };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([600.0, 470.0])
            .with_resizable(false)
            .with_maximize_button(false)
            .with_icon(Arc::new(icon)),
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        title,
        options,
        Box::new(move |cc| {
            let mut v = egui::Visuals::dark();
            v.panel_fill = BG;
            v.override_text_color = Some(TEXT);
            cc.egui_ctx.set_visuals(v);
            let mut style = (*cc.egui_ctx.style()).clone();
            style.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.5));
            style.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.5));
            cc.egui_ctx.set_style(style);
            Ok(Box::new(SetupApp {
                uninstall_mode,
                page: Page::Welcome,
                opt: Options { desktop: true, autostart: false, launch: true },
                remove_data: false,
                previous: installed_version(),
            }))
        }),
    )
}
