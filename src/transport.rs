use hidapi::{HidApi, HidDevice};
use std::collections::HashSet;
use std::thread::sleep;
use std::time::Duration;

pub const VID: u16 = 0x0c45;
pub const PID: u16 = 0x8006;
pub const CONFIG_INTERFACE: i32 = 0;
pub const VENDOR_INTERFACE: i32 = 1;

pub const CMD_BEGIN: u8 = 0x18;
pub const CMD_COMMIT: u8 = 0x02;
pub const CMD_APPLY: u8 = 0xf0;

pub const CMD_BASE_KEYMAP: u8 = 0x11;
pub const CMD_FN_KEYMAP: u8 = 0x27;
pub const CMD_RGB: u8 = 0x13;
pub const CMD_SMART_WHEEL: u8 = 0x25;

pub struct InterfaceHandle {
    pub interface_number: i32,
    pub path: String,
    pub dev: HidDevice,
}

pub struct InterfaceSummary {
    pub interface_number: i32,
    pub usage_page: u16,
    pub usage: u16,
    pub path: String,
}

pub fn list_interfaces() -> Result<Vec<InterfaceSummary>, String> {
    let api = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
    let mut out = Vec::new();
    for d in api
        .device_list()
        .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
    {
        out.push(InterfaceSummary {
            interface_number: d.interface_number(),
            usage_page: d.usage_page(),
            usage: d.usage(),
            path: d.path().to_string_lossy().into_owned(),
        });
    }
    Ok(out)
}

pub fn open_all_interfaces() -> Result<Vec<InterfaceHandle>, String> {
    let api = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
    let mut handles = Vec::new();
    let mut seen_paths = HashSet::new();
    for d in api
        .device_list()
        .filter(|d| d.vendor_id() == VID && d.product_id() == PID)
    {
        let path = d.path().to_string_lossy().into_owned();
        if !seen_paths.insert(path.clone()) {
            continue;
        }
        match d.open_device(&api) {
            Ok(dev) => {
                let _ = dev.set_blocking_mode(false);
                handles.push(InterfaceHandle {
                    interface_number: d.interface_number(),
                    path,
                    dev,
                });
            }
            Err(e) => {
                eprintln!(
                    "Warning: could not open iface {} ({path}): {e}",
                    d.interface_number()
                );
            }
        }
    }
    Ok(handles)
}

/// Open the C989 vendor/media HID interface (Interface 1) for listening to Report ID 0x05 triggers.
pub fn open_vendor_interface() -> Result<InterfaceHandle, String> {
    let api = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == VID
                && d.product_id() == PID
                && d.interface_number() == VENDOR_INTERFACE
                && d.usage_page() == 0xffff
        })
        .or_else(|| {
            api.device_list().find(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.interface_number() == VENDOR_INTERFACE
            })
        })
        .ok_or_else(|| {
            "Cloud Nine C989 (0c45:8006, interface 1) not found. Check connection/permissions."
                .to_string()
        })?;

    let path = info.path().to_string_lossy().into_owned();
    let dev = info
        .open_device(&api)
        .map_err(|e| format!("Failed to open interface 1 ({path}): {e}"))?;
    Ok(InterfaceHandle {
        interface_number: info.interface_number(),
        path,
        dev,
    })
}

pub struct C989 {
    dev: HidDevice,
}

impl C989 {
    pub fn open() -> Result<Self, String> {
        let api = HidApi::new().map_err(|e| format!("HidApi init failed: {e}"))?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == VID
                    && d.product_id() == PID
                    && d.interface_number() == CONFIG_INTERFACE
            })
            .ok_or_else(|| {
                "Cloud Nine C989 (0c45:8006, interface 0) not found. Check connection/permissions."
                    .to_string()
            })?;

        let dev = info
            .open_device(&api)
            .map_err(|e| format!("Failed to open hidraw device (try sudo or udev rule): {e}"))?;

        Ok(Self { dev })
    }

    pub fn send_report(&self, payload: &[u8; 64]) -> Result<(), String> {
        let mut buf = [0u8; 65];
        buf[0] = 0x00;
        buf[1..].copy_from_slice(payload);
        self.dev
            .send_feature_report(&buf)
            .map_err(|e| format!("send_feature_report failed: {e}"))?;
        Ok(())
    }

    pub fn get_report(&self) -> Result<[u8; 64], String> {
        let mut buf = [0u8; 65];
        buf[0] = 0x00;
        self.dev
            .get_feature_report(&mut buf)
            .map_err(|e| format!("get_feature_report failed: {e}"))?;
        let mut out = [0u8; 64];
        out.copy_from_slice(&buf[1..]);
        Ok(out)
    }

    pub fn send_cmd_with_ack(&self, label: &str, payload: &[u8; 64]) -> Result<[u8; 64], String> {
        self.send_report(payload)?;
        for _ in 0..10 {
            sleep(Duration::from_millis(35));
            let ack = self.get_report()?;
            if ack[0] != 0xff {
                return Ok(ack);
            }
        }
        Err(format!("{label}: keyboard stayed busy (0xFF)"))
    }

    pub fn write_block(&self, cmd: u8, pages: &[[u8; 64]], send_apply: bool) -> Result<(), String> {
        let mut begin = [0u8; 64];
        begin[0] = 0x04;
        begin[1] = CMD_BEGIN;
        self.send_cmd_with_ack("BEGIN", &begin)?;

        let mut header = [0u8; 64];
        header[0] = 0x04;
        header[1] = cmd;
        header[8] = pages.len() as u8;
        self.send_cmd_with_ack("HEADER", &header)?;

        for page in pages {
            self.send_report(page)?;
            sleep(Duration::from_millis(15));
        }

        let mut commit = [0u8; 64];
        commit[0] = 0x04;
        commit[1] = CMD_COMMIT;
        self.send_cmd_with_ack("COMMIT", &commit)?;

        if send_apply {
            let mut apply = [0u8; 64];
            apply[0] = 0x04;
            apply[1] = CMD_APPLY;
            self.send_cmd_with_ack("APPLY", &apply)?;
        }

        Ok(())
    }

    pub fn write_base_keymap(&self, pages: &[[u8; 64]; 10]) -> Result<(), String> {
        self.write_block(CMD_BASE_KEYMAP, pages, true)
    }

    pub fn write_fn_keymap(&self, pages: &[[u8; 64]; 10]) -> Result<(), String> {
        self.write_block(CMD_FN_KEYMAP, pages, true)
    }

    pub fn write_rgb(&self, pages: &[[u8; 64]; 18]) -> Result<(), String> {
        self.write_block(CMD_RGB, pages, true)
    }

    pub fn write_smart_wheel(&self, page: &[u8; 64]) -> Result<(), String> {
        self.write_block(CMD_SMART_WHEEL, std::slice::from_ref(page), false)
    }
}
