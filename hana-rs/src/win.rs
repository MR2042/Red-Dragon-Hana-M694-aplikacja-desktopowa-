//! Funkcje systemowe Windows: chowanie/pokazywanie okna, jedna kopia aplikacji,
//! autostart, ustawienia aplikacji w rejestrze i powiadomienia Windows.

use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError};
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SW_HIDE, SW_RESTORE, SW_SHOW, SetForegroundWindow, ShowWindow,
};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

pub const WINDOW_TITLE: &str = "Hana Control – Redragon M694";
const APP_ID: &str = "Redragon.HanaControl";
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "HanaControl";
const SETTINGS_KEY: &str = r"Software\HanaControl";

/// Próg powiadomienia o niskiej baterii (%).
pub const LOW_BATTERY: u8 = 10;

static HWND: AtomicIsize = AtomicIsize::new(0);
static LOW_BATTERY_NOTIFY: AtomicBool = AtomicBool::new(true);

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// ---------- okno ----------

pub fn set_hwnd(h: isize) {
    HWND.store(h, Ordering::SeqCst);
}

fn hwnd() -> *mut c_void {
    HWND.load(Ordering::SeqCst) as *mut c_void
}

pub fn hide_window() {
    let h = hwnd();
    if !h.is_null() {
        unsafe { ShowWindow(h, SW_HIDE) };
    }
}

pub fn show_window() {
    let h = hwnd();
    if !h.is_null() {
        bring_to_front(h);
    }
}

fn bring_to_front(h: *mut c_void) {
    unsafe {
        ShowWindow(h, SW_SHOW);
        if IsIconic(h) != 0 {
            ShowWindow(h, SW_RESTORE);
        }
        SetForegroundWindow(h);
    }
}

/// Zwraca false, jeśli aplikacja już działa – wtedy pokazuje jej okno.
pub fn ensure_single_instance() -> bool {
    unsafe {
        let name = wide("Local\\HanaControl_M694");
        let _mutex = CreateMutexW(std::ptr::null(), 0, name.as_ptr()); // uchwyt celowo zostaje otwarty
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let title = wide(WINDOW_TITLE);
            let h = FindWindowW(std::ptr::null(), title.as_ptr());
            if !h.is_null() {
                bring_to_front(h);
            }
            return false;
        }
    }
    true
}

// ---------- autostart ----------

fn autostart_command() -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    Some(format!("\"{}\" --tray", exe.display()))
}

pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

pub fn set_autostart(on: bool) -> std::io::Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if on {
        let cmd = autostart_command().ok_or_else(|| std::io::Error::other("brak ścieżki programu"))?;
        key.set_value(RUN_VALUE, &cmd)
    } else {
        match key.delete_value(RUN_VALUE) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    }
}

/// Jeśli autostart jest włączony, aktualizuje ścieżkę (np. po przeniesieniu pliku .exe).
pub fn refresh_autostart_path() {
    if autostart_enabled() {
        let _ = set_autostart(true);
    }
}

// ---------- ustawienia aplikacji ----------

pub fn load_settings() {
    let on = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(SETTINGS_KEY)
        .and_then(|k| k.get_value::<u32, _>("LowBatteryNotify"))
        .map(|v| v != 0)
        .unwrap_or(true);
    LOW_BATTERY_NOTIFY.store(on, Ordering::SeqCst);
}

pub fn low_battery_notify() -> bool {
    LOW_BATTERY_NOTIFY.load(Ordering::SeqCst)
}

pub fn set_low_battery_notify(on: bool) {
    LOW_BATTERY_NOTIFY.store(on, Ordering::SeqCst);
    if let Ok((k, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(SETTINGS_KEY) {
        let _ = k.set_value("LowBatteryNotify", &(on as u32));
    }
}

// ---------- powiadomienia Windows ----------

fn data_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("HanaControl"))
}

/// Rejestruje nazwę i ikonę aplikacji dla powiadomień (bez instalatora).
pub fn register_notifications() {
    let Some(dir) = data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let icon = dir.join("icon.png");
    let _ = std::fs::write(&icon, include_bytes!("../assets/icon64.png"));
    if let Ok((k, _)) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(format!(r"Software\Classes\AppUserModelId\{APP_ID}")) {
        let _ = k.set_value("DisplayName", &"Hana Control");
        let _ = k.set_value("IconUri", &icon.display().to_string());
    }
}

pub fn notify(title: &str, text: &str) {
    let _ = try_notify(title, text);
}

/// Wysyła powiadomienie i zwraca błąd, jeśli Windows go nie przyjął.
pub fn try_notify(title: &str, text: &str) -> Result<(), String> {
    use tauri_winrt_notification::{Sound, Toast};
    // nazwa i ikona aplikacji muszą być zarejestrowane, inaczej Windows po cichu odrzuca powiadomienie
    register_notifications();
    Toast::new(APP_ID)
        .title(title)
        .text1(text)
        .sound(Some(Sound::Default))
        .show()
        .map_err(|e| format!("Windows odrzucił powiadomienie: {e}"))
}

/// Powód, dla którego Windows może teraz nie pokazywać dymków powiadomień (None = pokazuje).
pub fn notifications_paused_reason() -> Option<&'static str> {
    use windows_sys::Win32::UI::Shell::SHQueryUserNotificationState;
    let mut state = 0;
    if unsafe { SHQueryUserNotificationState(&mut state) } != 0 {
        return None;
    }
    match state {
        1 => Some("ekran jest zablokowany albo wygaszony"),
        2 | 3 => Some("działa aplikacja pełnoekranowa (np. gra)"),
        4 => Some("włączony jest tryb prezentacji"),
        6 => Some("Windows jest w trybie cichym"),
        _ => None,
    }
}
