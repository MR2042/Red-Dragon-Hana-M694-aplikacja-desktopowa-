//! Wątek w tle: łączy się z myszą, czyta/zapisuje konfigurację, sprawdza baterię
//! i przełącza połączenie po podłączeniu/odłączeniu kabla.

use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread;
use std::time::Duration;

use hidapi::HidApi;

use crate::config::Config;
use crate::device::{Battery, Mouse};
use crate::tray::TrayMsg;
use crate::win;

pub enum Request {
    Reload,
    Write(Config),
}

pub enum Event {
    Connected { wired: bool, config: Config, battery: Option<Battery> },
    Disconnected(String),
    Battery(Battery),
    Written { config: Config, verified: bool },
    Error(String),
}

pub fn spawn(ctx: eframe::egui::Context, tray: Sender<TrayMsg>) -> (Sender<Request>, Receiver<Event>) {
    let (req_tx, req_rx) = mpsc::channel::<Request>();
    let (ev_tx, ev_rx) = mpsc::channel::<Event>();
    thread::spawn(move || run(req_rx, ev_tx, ctx, tray));
    (req_tx, ev_rx)
}

/// Pilnuje, żeby powiadomienie o niskiej baterii pojawiło się raz na rozładowanie.
struct LowBatteryWatch {
    notified: bool,
}

impl LowBatteryWatch {
    fn check(&mut self, b: &Battery) {
        if b.charging != 0 || b.percent > win::LOW_BATTERY + 5 {
            self.notified = false; // ładuje się albo naładowana – kolejne rozładowanie znów powiadomi
            return;
        }
        if b.percent <= win::LOW_BATTERY && !self.notified && win::low_battery_notify() {
            self.notified = true;
            win::notify(
                "Niski poziom baterii myszy",
                &format!("Redragon M694: zostało {}%. Podłącz mysz kablem USB, aby ją naładować.", b.percent),
            );
        }
    }
}

fn run(rx: Receiver<Request>, tx: Sender<Event>, ctx: eframe::egui::Context, tray: Sender<TrayMsg>) {
    let mut watch = LowBatteryWatch { notified: false };
    let send = |e: Event, watch: &mut LowBatteryWatch| {
        match &e {
            Event::Connected { battery: Some(b), .. } | Event::Battery(b) => {
                let _ = tray.send(TrayMsg::Battery(*b));
                watch.check(b);
            }
            Event::Disconnected(_) => {
                let _ = tray.send(TrayMsg::Disconnected);
            }
            _ => {}
        }
        let _ = tx.send(e);
        ctx.request_repaint();
    };
    let mut api = match HidApi::new() {
        Ok(a) => a,
        Err(e) => {
            send(Event::Disconnected(format!("Nie można uruchomić obsługi USB HID: {e}")), &mut watch);
            return;
        }
    };
    let mut mouse: Option<Mouse> = None;
    let mut last_wired_present: Option<bool> = None;

    // Połącz i wczytaj konfigurację.
    let connect = |api: &mut HidApi, mouse: &mut Option<Mouse>| -> Event {
        *mouse = None;
        match Mouse::open(api) {
            Ok(m) => match m.read_config() {
                Ok(raw) => {
                    let battery = m.battery().ok();
                    let wired = m.wired;
                    *mouse = Some(m);
                    Event::Connected { wired, config: Config { raw }, battery }
                }
                Err(e) => Event::Disconnected(e),
            },
            Err(e) => Event::Disconnected(e),
        }
    };

    send(connect(&mut api, &mut mouse), &mut watch);

    loop {
        match rx.recv_timeout(Duration::from_secs(4)) {
            Ok(Request::Reload) => send(connect(&mut api, &mut mouse), &mut watch),
            Ok(Request::Write(cfg)) => {
                let Some(m) = mouse.as_ref() else {
                    send(Event::Error("Mysz nie jest połączona.".into()), &mut watch);
                    continue;
                };
                let result = m.write_config(&cfg.raw).and_then(|_| {
                    thread::sleep(Duration::from_millis(150));
                    m.read_config()
                });
                match result {
                    Ok(back) => send(Event::Written { verified: back == cfg.raw, config: Config { raw: back } }, &mut watch),
                    Err(e) => {
                        mouse = None;
                        send(Event::Disconnected(e), &mut watch);
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                let wired_present = Mouse::wired_present(&mut api);
                let cable_changed = last_wired_present.is_some_and(|w| w != wired_present);
                last_wired_present = Some(wired_present);

                match mouse.as_ref() {
                    None => {
                        let ev = connect(&mut api, &mut mouse);
                        // Bez myszy nie zasypujemy okna komunikatami – wysyłamy tylko udane połączenie.
                        if matches!(ev, Event::Connected { .. }) {
                            send(ev, &mut watch);
                        }
                    }
                    Some(m) if cable_changed && m.wired != wired_present => {
                        send(connect(&mut api, &mut mouse), &mut watch);
                    }
                    Some(m) => match m.battery() {
                        Ok(b) => send(Event::Battery(b), &mut watch),
                        Err(e) => {
                            mouse = None;
                            send(Event::Disconnected(e), &mut watch);
                        }
                    },
                }
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}
