//! Komunikacja z myszą przez protokół "Evision" (układ Telink, VID 0x320F).
//! Kanał: kolekcja HID producenta 0xFF1C / 0x92, raport 0x04, 63 bajty danych.
//! Pakiet: [04] 00 00 CMD LEN OFF_LO OFF_HI 00 [dane…]; odpowiedź ma ten sam nagłówek, dane od bajtu 7.

use std::time::{Duration, Instant};

use hidapi::{HidApi, HidDevice};

use crate::config::CONFIG_SIZE;

pub const VENDOR_ID: u16 = 0x320F;
pub const PID_WIRED: u16 = 0x226E; // mysz po kablu USB ("3-mode mouse")
pub const PID_RECEIVER: u16 = 0x226F; // odbiornik 2.4G
const CONFIG_USAGE_PAGE: u16 = 0xFF1C;

const CMD_BEGIN: u8 = 0x01;
const CMD_END: u8 = 0x02;
const CMD_READ_CFG: u8 = 0x05;
const CMD_WRITE_CFG: u8 = 0x06;
const CMD_BATTERY: u8 = 0x1A;
const CMD_ONLINE: u8 = 0xAA;

pub struct Mouse {
    dev: HidDevice,
    chunk: u8,
    pub wired: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Battery {
    pub percent: u8,
    /// 0 – nie ładuje, 1 – ładuje, 2 – naładowana
    pub charging: u8,
}

pub type Result<T> = std::result::Result<T, String>;

impl Mouse {
    /// Otwiera pierwszą odpowiadającą mysz – najpierw kabel (wtedy odbiornik milczy), potem odbiornik.
    pub fn open(api: &mut HidApi) -> Result<Mouse> {
        let _ = api.refresh_devices();
        for (pid, chunk) in [(PID_WIRED, 0x38u8), (PID_RECEIVER, 0x18u8)] {
            let infos: Vec<_> = api
                .device_list()
                .filter(|d| d.vendor_id() == VENDOR_ID && d.product_id() == pid && d.usage_page() == CONFIG_USAGE_PAGE)
                .cloned()
                .collect();
            for info in infos {
                if let Ok(dev) = info.open_device(api) {
                    let m = Mouse { dev, chunk, wired: pid == PID_WIRED };
                    if m.is_online().unwrap_or(false) {
                        return Ok(m);
                    }
                }
            }
        }
        Err("Nie znaleziono myszy M694. Podłącz kabel lub odbiornik 2.4G i rusz myszką, aby ją wybudzić.".into())
    }

    /// Czy kabel z myszą jest podłączony (bez otwierania urządzenia).
    pub fn wired_present(api: &mut HidApi) -> bool {
        let _ = api.refresh_devices();
        api.device_list().any(|d| d.vendor_id() == VENDOR_ID && d.product_id() == PID_WIRED)
    }

    fn xfer(&self, data: &[u8]) -> Result<[u8; 63]> {
        let mut pkt = [0u8; 64];
        pkt[0] = 0x04;
        pkt[1..1 + data.len()].copy_from_slice(data);
        self.dev.write(&pkt).map_err(|e| format!("Błąd zapisu do myszy: {e}"))?;

        let deadline = Instant::now() + Duration::from_millis(1500);
        let mut buf = [0u8; 64];
        while Instant::now() < deadline {
            let n = self.dev.read_timeout(&mut buf, 100).map_err(|e| format!("Błąd odczytu z myszy: {e}"))?;
            if n >= 8 && buf[0] == 0x04 && buf[3] == data[2] {
                let mut r = [0u8; 63];
                r.copy_from_slice(&buf[1..64]);
                return Ok(r);
            }
        }
        Err(format!("Mysz nie odpowiedziała (komenda 0x{:02X}).", data[2]))
    }

    pub fn is_online(&self) -> Result<bool> {
        // Odbiornik zgłasza stan w bajcie 7, mysz po kablu w bajcie 6.
        let r = self.xfer(&[0, 0, CMD_ONLINE, self.chunk, 0, 0])?;
        Ok(r[6] | r[7] != 0)
    }

    pub fn battery(&self) -> Result<Battery> {
        let r = self.xfer(&[0, 0, CMD_BATTERY, self.chunk, 0, 0])?;
        Ok(Battery { percent: r[7], charging: r[8] })
    }

    pub fn read_config(&self) -> Result<[u8; CONFIG_SIZE]> {
        let mut block = [0u8; CONFIG_SIZE];
        self.xfer(&[0, 0, CMD_BEGIN, self.chunk, 0, 0])?;
        let mut off = 0usize;
        while off < CONFIG_SIZE {
            let n = (self.chunk as usize).min(CONFIG_SIZE - off);
            let r = self.xfer(&[0, 0, CMD_READ_CFG, n as u8, off as u8, 0, 0])?;
            block[off..off + n].copy_from_slice(&r[7..7 + n]);
            off += n;
        }
        self.xfer(&[0, 0, CMD_END, self.chunk, 0, 0])?;
        Ok(block)
    }

    pub fn write_config(&self, block: &[u8; CONFIG_SIZE]) -> Result<()> {
        self.xfer(&[0, 0, CMD_BEGIN, self.chunk, 0, 0])?;
        let mut off = 0usize;
        while off < CONFIG_SIZE {
            let n = (self.chunk as usize).min(CONFIG_SIZE - off);
            let mut pkt = vec![0u8, 0, CMD_WRITE_CFG, n as u8, off as u8, 0, 0];
            pkt.extend_from_slice(&block[off..off + n]);
            self.xfer(&pkt)?;
            std::thread::sleep(Duration::from_millis(20));
            off += n;
        }
        self.xfer(&[0, 0, CMD_END, self.chunk, 0, 0])?;
        Ok(())
    }
}
