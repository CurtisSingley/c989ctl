use hidapi::{HidApi, HidDevice};
use std::env;
use std::thread::sleep;
use std::time::Duration;

const VID: u16 = 0x0c45;
const PID: u16 = 0x8006;
const CONFIG_INTERFACE: i32 = 0;

struct C989 {
    dev: HidDevice,
}

impl C989 {
    fn open() -> Result<Self, String> {
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

    /// Sends a 64-byte HID Feature Report (prefixed with Report ID 0x00).
    fn send_report(&self, payload: &[u8; 64]) -> Result<(), String> {
        let mut buf = [0u8; 65];
        buf[0] = 0x00; // Report ID 0
        buf[1..].copy_from_slice(payload);
        self.dev
            .send_feature_report(&buf)
            .map_err(|e| format!("send_feature_report failed: {e}"))?;
        Ok(())
    }

    /// Reads a 64-byte HID Feature Report acknowledgment.
    fn get_report(&self) -> Result<[u8; 64], String> {
        let mut buf = [0u8; 65];
        buf[0] = 0x00; // Report ID 0
        self.dev
            .get_feature_report(&mut buf)
            .map_err(|e| format!("get_feature_report failed: {e}"))?;
        let mut out = [0u8; 64];
        out.copy_from_slice(&buf[1..]);
        Ok(out)
    }

    /// Sends a command packet and polls GET_REPORT until the keyboard isn't busy (0xFF).
    fn send_cmd_with_ack(&self, label: &str, payload: &[u8; 64]) -> Result<[u8; 64], String> {
        self.send_report(payload)?;
        for attempt in 0..10 {
            sleep(Duration::from_millis(35));
            let ack = self.get_report()?;
            println!("{label} ack (try {attempt}): {:02x?}", &ack[..10]);
            if ack[0] != 0xff {
                return Ok(ack);
            }
        }
        Err(format!("{label}: keyboard stayed busy (0xFF)"))
    }

    /// Executes the 4-stage Sonix transaction: 04 18 -> 04 <cmd> -> data -> 04 02 -> 04 f0
    fn write_block(&self, cmd: u8, pages: &[[u8; 64]]) -> Result<(), String> {
        // 1. Begin transaction (04 18)
        let mut begin = [0u8; 64];
        begin[0] = 0x04;
        begin[1] = 0x18;
        self.send_cmd_with_ack("BEGIN", &begin)?;

        // 2. Command header + page count (04 <cmd> ... <count>)
        let mut header = [0u8; 64];
        header[0] = 0x04;
        header[1] = cmd;
        header[8] = pages.len() as u8;
        self.send_cmd_with_ack("HEADER", &header)?;

        // 3. Raw 64-byte data pages
        for page in pages {
            self.send_report(page)?;
            sleep(Duration::from_millis(15));
        }

        // 4. Commit (04 02) & Apply (04 f0)
        let mut commit = [0u8; 64];
        commit[0] = 0x04;
        commit[1] = 0x02;
        self.send_cmd_with_ack("COMMIT", &commit)?;

        let mut apply = [0u8; 64];
        apply[0] = 0x04;
        apply[1] = 0xf0;
        self.send_cmd_with_ack("APPLY", &apply)?;

        Ok(())
    }

    /// Sets the keyboard to Static RGB mode (0x01) with the given (R, G, B) color.
    fn set_static_rgb(&self, r: u8, g: u8, b: u8) -> Result<(), String> {
        let mut pages = [[0u8; 64]; 18];

        // Helper to build a 16-byte lighting mode slot
        let make_slot =
            |mode: u8, r: u8, g: u8, b: u8, flag: u8, bright: u8, speed: u8| -> [u8; 16] {
                [
                    mode, r, g, b, 0x00, 0x00, 0x00, 0x00, flag, bright, speed, 0x00, 0x00, 0x00,
                    0xaa, 0x55,
                ]
            };

        // Page 0: modes 01..04 (mode 01 = Static)
        let active_slot = make_slot(0x01, r, g, b, 0x00, 0x06, 0x06);
        pages[0][0..16].copy_from_slice(&active_slot);
        pages[0][16..32].copy_from_slice(&make_slot(0x02, 0x00, 0xff, 0xff, 0x00, 0x06, 0x06));
        pages[0][32..48].copy_from_slice(&make_slot(0x03, 0xff, 0x00, 0x00, 0x00, 0x06, 0x06));
        pages[0][48..64].copy_from_slice(&make_slot(0x04, 0xff, 0xff, 0x00, 0x00, 0x06, 0x06));

        // Pages 1..4: default slots for modes 05..13 and 80 (per-key)
        pages[1][0..16].copy_from_slice(&make_slot(0x05, 0x00, 0x00, 0xff, 0x00, 0x06, 0x06));
        pages[1][16..32].copy_from_slice(&make_slot(0x06, 0xa0, 0x20, 0xf0, 0x00, 0x06, 0x06));
        pages[1][32..48].copy_from_slice(&make_slot(0x07, 0xa0, 0x20, 0xf0, 0x01, 0x06, 0x06));
        pages[1][48..64].copy_from_slice(&make_slot(0x08, 0xa0, 0x20, 0xf0, 0x01, 0x06, 0x06));

        for (i, mode) in (0x09u8..=0x13u8).enumerate() {
            let page_idx = 2 + (i / 4);
            let offset = (i % 4) * 16;
            pages[page_idx][offset..offset + 16]
                .copy_from_slice(&make_slot(mode, 0xa0, 0x20, 0xf0, 0x01, 0x06, 0x06));
        }
        pages[4][48..64].copy_from_slice(&make_slot(0x80, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00));

        // Pages 5..7: zeros (already initialized)

        // Pages 8..16: per-key LED table (16 entries of 0x80 00 00 00 per page)
        for p in 8..=16 {
            for k in 0..16 {
                pages[p][k * 4] = 0x80;
            }
        }

        // Page 17: currently active lighting mode
        pages[17][0..16].copy_from_slice(&active_slot);

        self.write_block(0x13, &pages)
    }
}

fn parse_hex_color(hex: &str) -> Result<(u8, u8, u8), String> {
    let clean = hex.trim_start_matches('#');
    if clean.len() != 6 {
        return Err("Expected 6-digit hex color (e.g., ff0000 or 00ff88)".into());
    }
    let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| "Invalid red hex")?;
    let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| "Invalid green hex")?;
    let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| "Invalid blue hex")?;
    Ok((r, g, b))
}

fn main() -> Result<(), String> {
    let arg = env::args().nth(1).unwrap_or_else(|| "ff0000".to_string());

    let (r, g, b) = match arg.to_lowercase().as_str() {
        "red" => (0xff, 0x00, 0x00),
        "green" => (0x00, 0xff, 0x00),
        "blue" => (0x00, 0x00, 0xff),
        "white" => (0xff, 0xff, 0xff),
        "off" => (0x00, 0x00, 0x00),
        other => parse_hex_color(other)?,
    };

    let kb = C989::open()?;
    kb.set_static_rgb(r, g, b)?;
    println!("Successfully set C989 static RGB to #{r:02x}{g:02x}{b:02x}");
    Ok(())
}
