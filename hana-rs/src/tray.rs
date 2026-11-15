//! Ikona w zasobniku systemowym (obok zegara). Działa we własnym wątku z pętlą komunikatów
//! Windows, więc reaguje także wtedy, gdy okno aplikacji jest schowane.

use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Duration;

use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{DispatchMessageW, MSG, PM_REMOVE, PeekMessageW, TranslateMessage};

use crate::device::Battery;
use crate::win;

pub enum TrayMsg {
    Battery(Battery),
    Disconnected,
}

fn battery_text(b: &Battery) -> String {
    match b.charging {
        1 => format!("Bateria: {}% (ładowanie)", b.percent),
        2 => format!("Bateria: {}% (naładowana)", b.percent),
        _ => format!("Bateria: {}%", b.percent),
    }
}

pub fn spawn(ctx: eframe::egui::Context) -> Sender<TrayMsg> {
    let (tx, rx) = mpsc::channel::<TrayMsg>();
    thread::spawn(move || {
        let show = MenuItem::new("Pokaż Hana Control", true, None);
        let battery = MenuItem::new("Mysz niepołączona", false, None);
        let quit = MenuItem::new("Zamknij aplikację", true, None);
        let menu = Menu::new();
        let _ = menu.append_items(&[&show, &battery, &PredefinedMenuItem::separator(), &quit]);

        let icon = Icon::from_rgba(include_bytes!("../assets/icon32.rgba").to_vec(), 32, 32).expect("ikona");
        let tray = match TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("Hana Control")
            .with_icon(icon)
            .build()
        {
            Ok(t) => t,
            Err(_) => return,
        };

        loop {
            // pętla komunikatów Windows dla ikony i menu
            unsafe {
                let mut msg: MSG = std::mem::zeroed();
                while PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
                if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = ev {
                    win::show_window();
                    ctx.request_repaint();
                }
            }

            while let Ok(ev) = MenuEvent::receiver().try_recv() {
                if ev.id == *show.id() {
                    win::show_window();
                    ctx.request_repaint();
                } else if ev.id == *quit.id() {
                    drop(tray);
                    std::process::exit(0);
                }
            }

            while let Ok(m) = rx.try_recv() {
                let text = match &m {
                    TrayMsg::Battery(b) => battery_text(b),
                    TrayMsg::Disconnected => "Mysz niepołączona".to_string(),
                };
                battery.set_text(&text);
                let _ = tray.set_tooltip(Some(format!("Hana Control\n{text}")));
            }

            thread::sleep(Duration::from_millis(40));
        }
    });
    tx
}
