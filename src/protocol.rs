use crate::layout::{FN_FIRMWARE_DEFAULT_INDICES, KEYS, hid_usage_by_name};
use std::collections::BTreeMap;
use std::fmt;

pub const KEYMAP_PAGES: usize = 10;
pub const KEYMAP_SLOTS: usize = 160;
pub const RGB_PAGES: usize = 18;
pub const MAX_LIGHT_INDEX: usize = 144;

/// 4-byte key entry used in `04 11` (base) and `04 27` (Fn) keymaps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEntry {
    /// `00 00 00 00` — Firmware default.
    Default,
    /// `05 03 00 00` — Unassigned / disabled.
    Disabled,
    /// `02 <mod_bitmask> <hid_usage> 00` — Standard key or modifier combo.
    Key { mods: u8, usage: u8 },
    /// `01 01 <btn> 00` — Mouse button (0x01 left, 0x02 right, 0x04 middle).
    MouseButton(u8),
    /// `01 03 <dir> 00` — Mouse scroll (1 = up / 0x01, -1 = down / 0xff).
    MouseScroll(i8),
    /// `03 <lo> <hi> 00` — Consumer / media key (USB HID Usage Page 0x0C).
    Consumer(u16),
    /// `05 02 <slot> 00` — Macro slot reference.
    Macro(u8),
    /// Raw 4-byte entry.
    Raw([u8; 4]),
}

impl KeyEntry {
    pub fn to_bytes(self) -> [u8; 4] {
        match self {
            KeyEntry::Default => [0x00, 0x00, 0x00, 0x00],
            KeyEntry::Disabled => [0x05, 0x03, 0x00, 0x00],
            KeyEntry::Key { mods, usage } => [0x02, mods, usage, 0x00],
            KeyEntry::MouseButton(btn) => [0x01, 0x01, btn, 0x00],
            KeyEntry::MouseScroll(dir) => [0x01, 0x03, dir as u8, 0x00],
            KeyEntry::Consumer(code) => {
                let [lo, hi] = code.to_le_bytes();
                [0x03, lo, hi, 0x00]
            }
            KeyEntry::Macro(slot) => [0x05, 0x02, slot, 0x00],
            KeyEntry::Raw(bytes) => bytes,
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let trimmed = input.trim();
        let lower = trimmed.to_lowercase();
        let norm = lower.replace(['-', ' '], "_");

        match norm.as_str() {
            "default" | "reset" => return Ok(KeyEntry::Default),
            "disabled" | "none" | "unassigned" | "noop" => return Ok(KeyEntry::Disabled),
            "mouse_left" | "lclick" | "left_click" => return Ok(KeyEntry::MouseButton(0x01)),
            "mouse_right" | "rclick" | "right_click" => return Ok(KeyEntry::MouseButton(0x02)),
            "mouse_middle" | "mclick" | "middle_click" => return Ok(KeyEntry::MouseButton(0x04)),
            "scroll_up" | "wheel_up" => return Ok(KeyEntry::MouseScroll(1)),
            "scroll_down" | "wheel_down" => return Ok(KeyEntry::MouseScroll(-1)),
            "media_player" | "media" => return Ok(KeyEntry::Consumer(0x0183)),
            "play_pause" | "playpause" => return Ok(KeyEntry::Consumer(0x00cd)),
            "stop" | "media_stop" => return Ok(KeyEntry::Consumer(0x00b7)),
            "prev_track" | "previous_track" | "prev" => return Ok(KeyEntry::Consumer(0x00b6)),
            "next_track" | "next" => return Ok(KeyEntry::Consumer(0x00b5)),
            "mute" | "vol_mute" => return Ok(KeyEntry::Consumer(0x00e2)),
            "vol_up" | "volume_up" => return Ok(KeyEntry::Consumer(0x00e9)),
            "vol_down" | "volume_down" => return Ok(KeyEntry::Consumer(0x00ea)),
            _ => {}
        }

        if let Some(rest) = lower.strip_prefix("raw:") {
            let clean = rest.trim().replace(' ', "");
            if clean.len() != 8 {
                return Err(format!("raw entry must be 8 hex digits, got '{rest}'"));
            }
            let mut bytes = [0u8; 4];
            for i in 0..4 {
                bytes[i] = u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16)
                    .map_err(|_| format!("invalid hex in raw entry '{rest}'"))?;
            }
            return Ok(KeyEntry::Raw(bytes));
        }

        if let Some(rest) = lower.strip_prefix("macro:") {
            let slot = parse_u8_auto(rest.trim())
                .map_err(|_| format!("invalid macro slot '{rest}'"))?;
            return Ok(KeyEntry::Macro(slot));
        }

        if let Some(rest) = lower.strip_prefix("consumer:") {
            let code = parse_u16_auto(rest.trim())
                .map_err(|_| format!("invalid consumer usage code '{rest}'"))?;
            return Ok(KeyEntry::Consumer(code));
        }

        // Standard key or modifier combo: e.g. "a", "ctrl+c", "lctrl+shift+t"
        let parts: Vec<&str> = lower.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return Err("empty key action".into());
        }

        if parts.len() == 1 {
            let usage = hid_usage_by_name(parts[0])
                .ok_or_else(|| format!("unknown key action '{trimmed}'"))?;
            return Ok(KeyEntry::Key { mods: 0, usage });
        }

        let mut mods = 0u8;
        for &m in &parts[..parts.len() - 1] {
            let bit = match m.replace('-', "_").as_str() {
                "ctrl" | "lctrl" | "left_ctrl" => 0x01,
                "shift" | "lshift" | "left_shift" => 0x02,
                "alt" | "lalt" | "left_alt" => 0x04,
                "win" | "lwin" | "super" | "lsuper" | "gui" | "lgui" | "meta" => 0x08,
                "rctrl" | "right_ctrl" => 0x10,
                "rshift" | "right_shift" => 0x20,
                "ralt" | "right_alt" | "altgr" => 0x40,
                "rwin" | "rsuper" | "rgui" | "right_win" => 0x80,
                other => return Err(format!("unknown modifier '{other}' in '{trimmed}'")),
            };
            mods |= bit;
        }

        let key_part = parts[parts.len() - 1];
        let usage = hid_usage_by_name(key_part)
            .ok_or_else(|| format!("unknown key '{key_part}' in combo '{trimmed}'"))?;
        Ok(KeyEntry::Key { mods, usage })
    }
}

fn parse_u8_auto(s: &str) -> Result<u8, std::num::ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x") {
        u8::from_str_radix(hex, 16)
    } else {
        s.parse::<u8>()
    }
}

fn parse_u16_auto(s: &str) -> Result<u16, std::num::ParseIntError> {
    if let Some(hex) = s.strip_prefix("0x") {
        u16::from_str_radix(hex, 16)
    } else {
        s.parse::<u16>()
    }
}

pub fn set_keymap_slot(pages: &mut [[u8; 64]; KEYMAP_PAGES], key_index: usize, entry: KeyEntry) {
    assert!(key_index < KEYMAP_SLOTS, "key_index {key_index} out of range");
    let page = key_index / 16;
    let offset = (key_index % 16) * 4;
    pages[page][offset..offset + 4].copy_from_slice(&entry.to_bytes());
}

#[cfg(test)]
pub fn get_keymap_slot(pages: &[[u8; 64]; KEYMAP_PAGES], key_index: usize) -> [u8; 4] {
    let page = key_index / 16;
    let offset = (key_index % 16) * 4;
    pages[page][offset..offset + 4].try_into().unwrap()
}

/// Build a 10-page base keymap (`04 11`) from `(key_index, KeyEntry)` overrides.
/// Unspecified slots default to `00 00 00 00` (firmware default).
pub fn build_base_keymap(entries: &[(usize, KeyEntry)]) -> [[u8; 64]; KEYMAP_PAGES] {
    let mut pages = [[0u8; 64]; KEYMAP_PAGES];
    for &(idx, entry) in entries {
        set_keymap_slot(&mut pages, idx, entry);
    }
    pages[9][62] = 0xaa;
    pages[9][63] = 0x55;
    pages
}

/// Build a 10-page Fn layer keymap (`04 27`) from `(key_index, KeyEntry)` overrides.
/// Physical keys not in `FN_FIRMWARE_DEFAULT_INDICES` initialize to `05 03 00 00` (disabled),
/// while reserved keys and non-physical matrix slots remain `00 00 00 00`, matching official captures.
pub fn build_fn_keymap(entries: &[(usize, KeyEntry)]) -> [[u8; 64]; KEYMAP_PAGES] {
    let mut pages = [[0u8; 64]; KEYMAP_PAGES];
    for k in KEYS {
        if !FN_FIRMWARE_DEFAULT_INDICES.contains(&k.key_index) {
            set_keymap_slot(&mut pages, k.key_index, KeyEntry::Disabled);
        }
    }
    for &(idx, entry) in entries {
        set_keymap_slot(&mut pages, idx, entry);
    }
    pages[9][62] = 0xaa;
    pages[9][63] = 0x55;
    pages
}

/// User's verified safe baseline base layer (`04 11`).
pub fn baseline_base_entries() -> Vec<(usize, KeyEntry)> {
    vec![
        (18, KeyEntry::MouseScroll(1)),    // Copy       -> Scroll Up
        (36, KeyEntry::MouseScroll(-1)),   // Paste      -> Scroll Down
        (53, KeyEntry::MouseButton(0x01)), // M3         -> Mouse Left
        (54, KeyEntry::MouseButton(0x02)), // Cut        -> Mouse Right
        (71, KeyEntry::MouseButton(0x04)), // M4         -> Mouse Middle
        (90, KeyEntry::Key { mods: 0, usage: 0x39 }), // Select all -> Caps Lock
        (94, KeyEntry::Key { mods: 0, usage: 0x2a }), // Left Space -> Backspace
    ]
}

pub fn baseline_base_keymap() -> [[u8; 64]; KEYMAP_PAGES] {
    build_base_keymap(&baseline_base_entries())
}

/// User's captured baseline Fn layer (`04 27` from `filtered_apply_default_profile.txt`).
pub fn baseline_fn_entries() -> Vec<(usize, KeyEntry)> {
    vec![
        (2, KeyEntry::Consumer(0x0183)),              // F1 -> Media Player
        (3, KeyEntry::Consumer(0x00cd)),              // F2 -> Play/Pause
        (4, KeyEntry::Consumer(0x00b7)),              // F3 -> Stop
        (5, KeyEntry::Consumer(0x00b6)),              // F4 -> Prev Track
        (6, KeyEntry::Consumer(0x00b5)),              // F5 -> Next Track
        (39, KeyEntry::MouseButton(0x04)),            // W -> Mouse Middle
        (40, KeyEntry::Key { mods: 0, usage: 0x52 }), // E -> Up
        (41, KeyEntry::MouseButton(0x02)),            // R -> Mouse Right
        (42, KeyEntry::MouseScroll(1)),               // T -> Scroll Up
        (45, KeyEntry::MouseButton(0x02)),            // I -> Mouse Right
        (46, KeyEntry::Key { mods: 0, usage: 0x52 }), // O -> Up
        (47, KeyEntry::MouseButton(0x01)),            // P -> Mouse Left
        (48, KeyEntry::MouseScroll(1)),               // [ -> Scroll Up
        (57, KeyEntry::Key { mods: 0, usage: 0x50 }), // S -> Left
        (58, KeyEntry::Key { mods: 0, usage: 0x51 }), // D -> Down
        (59, KeyEntry::Key { mods: 0, usage: 0x4f }), // F -> Right
        (60, KeyEntry::MouseScroll(-1)),              // G -> Scroll Down
        (61, KeyEntry::Raw([0x02, 0x00, 0x01, 0x00])), // H
        (62, KeyEntry::Raw([0x02, 0x00, 0x01, 0x00])), // J
        (63, KeyEntry::Key { mods: 0, usage: 0x50 }), // K -> Left
        (64, KeyEntry::Key { mods: 0, usage: 0x51 }), // L -> Down
        (65, KeyEntry::Key { mods: 0, usage: 0x4f }), // ; -> Right
        (66, KeyEntry::MouseScroll(-1)),              // ' -> Scroll Down
        (79, KeyEntry::Raw([0x02, 0x00, 0x01, 0x00])), // N
        (80, KeyEntry::Key { mods: 0, usage: 0x50 }), // M -> Left
        (81, KeyEntry::Key { mods: 0, usage: 0x51 }), // , -> Down
        (82, KeyEntry::Key { mods: 0, usage: 0x4f }), // . -> Right
        (83, KeyEntry::MouseButton(0x04)),            // / -> Mouse Middle
        (94, KeyEntry::MouseButton(0x01)),            // Left Space -> Mouse Left
        (109, KeyEntry::MouseButton(0x01)),           // Right Space -> Mouse Left
        (112, KeyEntry::Key { mods: 0, usage: 0x47 }), // PrtSc -> Scroll Lock
        (115, KeyEntry::Key { mods: 0, usage: 0x49 }), // Pause -> Insert
    ]
}

pub fn baseline_fn_keymap() -> [[u8; 64]; KEYMAP_PAGES] {
    build_fn_keymap(&baseline_fn_entries())
}

// ============================================================================
// RGB Lighting (`04 13`, 18 pages = 1,152 bytes)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RgbMode {
    Static = 0x01,
    Colorful = 0x02,
    Falling = 0x03,
    Glittering = 0x04,
    Breath = 0x05,
    Spectrum = 0x06,
    Volcano = 0x07,
    Scrolling = 0x08,
    Rolling = 0x09,
    Pinwheel = 0x0a,
    Explode = 0x0b,
    Launch = 0x0c,
    Ripples = 0x0d,
    Flowing = 0x0e,
    SineWave = 0x0f,
    Tilt = 0x10,
    Shuttle = 0x11,
    SingleOn = 0x12,
    SingleOff = 0x13,
    UserDefine = 0x80,
}

pub const ALL_RGB_MODES: &[RgbMode] = &[
    RgbMode::Static,
    RgbMode::Colorful,
    RgbMode::Falling,
    RgbMode::Glittering,
    RgbMode::Breath,
    RgbMode::Spectrum,
    RgbMode::Volcano,
    RgbMode::Scrolling,
    RgbMode::Rolling,
    RgbMode::Pinwheel,
    RgbMode::Explode,
    RgbMode::Launch,
    RgbMode::Ripples,
    RgbMode::Flowing,
    RgbMode::SineWave,
    RgbMode::Tilt,
    RgbMode::Shuttle,
    RgbMode::SingleOn,
    RgbMode::SingleOff,
    RgbMode::UserDefine,
];

impl RgbMode {
    pub fn id(self) -> u8 {
        self as u8
    }

    pub fn slot_index(self) -> usize {
        match self {
            RgbMode::UserDefine => 19,
            other => (other as u8 - 1) as usize,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            RgbMode::Static => "static",
            RgbMode::Colorful => "colorful",
            RgbMode::Falling => "falling",
            RgbMode::Glittering => "glittering",
            RgbMode::Breath => "breath",
            RgbMode::Spectrum => "spectrum",
            RgbMode::Volcano => "volcano",
            RgbMode::Scrolling => "scrolling",
            RgbMode::Rolling => "rolling",
            RgbMode::Pinwheel => "pinwheel",
            RgbMode::Explode => "explode",
            RgbMode::Launch => "launch",
            RgbMode::Ripples => "ripples",
            RgbMode::Flowing => "flowing",
            RgbMode::SineWave => "sine_wave",
            RgbMode::Tilt => "tilt",
            RgbMode::Shuttle => "shuttle",
            RgbMode::SingleOn => "single_on",
            RgbMode::SingleOff => "single_off",
            RgbMode::UserDefine => "user_define",
        }
    }

    /// Default "full spectrum" byte (byte 8) observed in official captures for this mode.
    pub fn default_full_spectrum(self) -> bool {
        (0x07..=0x13).contains(&self.id())
    }

    pub fn from_u8(v: u8) -> Option<Self> {
        ALL_RGB_MODES.iter().copied().find(|m| m.id() == v)
    }

    pub fn parse(s: &str) -> Result<Self, String> {
        let norm = s.trim().to_lowercase().replace(['-', ' '], "_");
        match norm.as_str() {
            "static" | "static_color" | "solid" => Ok(RgbMode::Static),
            "colorful" => Ok(RgbMode::Colorful),
            "falling" => Ok(RgbMode::Falling),
            "glittering" => Ok(RgbMode::Glittering),
            "breath" | "breathing" => Ok(RgbMode::Breath),
            "spectrum" => Ok(RgbMode::Spectrum),
            "volcano" => Ok(RgbMode::Volcano),
            "scrolling" => Ok(RgbMode::Scrolling),
            "rolling" => Ok(RgbMode::Rolling),
            "pinwheel" => Ok(RgbMode::Pinwheel),
            "explode" => Ok(RgbMode::Explode),
            "launch" => Ok(RgbMode::Launch),
            "ripples" | "ripple" => Ok(RgbMode::Ripples),
            "flowing" => Ok(RgbMode::Flowing),
            "sine_wave" | "sinewave" | "sine" => Ok(RgbMode::SineWave),
            "tilt" => Ok(RgbMode::Tilt),
            "shuttle" => Ok(RgbMode::Shuttle),
            "single_on" | "singleon" => Ok(RgbMode::SingleOn),
            "single_off" | "singleoff" => Ok(RgbMode::SingleOff),
            "user_define" | "userdefine" | "per_key" | "perkey" | "custom" => Ok(RgbMode::UserDefine),
            other => {
                if let Ok(v) = parse_u8_auto(other) {
                    Self::from_u8(v).ok_or_else(|| format!("unsupported RGB mode id 0x{v:02x}"))
                } else {
                    Err(format!("unknown RGB mode '{s}'"))
                }
            }
        }
    }
}

impl fmt::Display for RgbMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (0x{:02x})", self.name(), self.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbConfig {
    pub mode: RgbMode,
    pub color: (u8, u8, u8),
    pub full_spectrum: bool,
    pub brightness: u8,
    pub speed: u8,
    pub per_key: BTreeMap<usize, (u8, u8, u8)>,
}

impl Default for RgbConfig {
    fn default() -> Self {
        Self {
            mode: RgbMode::Static,
            color: (0x00, 0x00, 0xff),
            full_spectrum: false,
            brightness: 5,
            speed: 6,
            per_key: BTreeMap::new(),
        }
    }
}

pub fn make_rgb_slot(
    mode: u8,
    (r, g, b): (u8, u8, u8),
    full_spectrum: bool,
    brightness: u8,
    speed: u8,
) -> [u8; 16] {
    [
        mode,
        r,
        g,
        b,
        0x00,
        0x00,
        0x00,
        0x00,
        u8::from(full_spectrum),
        brightness,
        speed,
        0x00,
        0x00,
        0x00,
        0xaa,
        0x55,
    ]
}

/// Build the 18-page RGB lighting block (`04 13`).
pub fn build_rgb_pages(cfg: &RgbConfig) -> [[u8; 64]; RGB_PAGES] {
    let mut pages = [[0u8; 64]; RGB_PAGES];

    // Default templates for all 20 slots (pages 0..=4) matching official captures
    let default_slots: [(u8, (u8, u8, u8), bool, u8, u8); 20] = [
        (0x01, (0xff, 0x00, 0x00), false, 0x06, 0x06),
        (0x02, (0x00, 0xff, 0xff), false, 0x06, 0x06),
        (0x03, (0xff, 0x00, 0x00), false, 0x06, 0x06),
        (0x04, (0xff, 0xff, 0x00), false, 0x06, 0x06),
        (0x05, (0x00, 0x00, 0xff), false, 0x06, 0x06),
        (0x06, (0xa0, 0x20, 0xf0), false, 0x06, 0x06),
        (0x07, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x08, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x09, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0a, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0b, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0c, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0d, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0e, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x0f, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x10, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x11, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x12, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x13, (0xa0, 0x20, 0xf0), true, 0x06, 0x06),
        (0x80, (0x00, 0x00, 0x00), false, 0x05, 0x00),
    ];

    for (i, &(mode, color, full_spec, bright, speed)) in default_slots.iter().enumerate() {
        let p = i / 4;
        let off = (i % 4) * 16;
        pages[p][off..off + 16]
            .copy_from_slice(&make_rgb_slot(mode, color, full_spec, bright, speed));
    }

    let active_slot = if cfg.mode == RgbMode::UserDefine {
        make_rgb_slot(0x80, (0, 0, 0), false, cfg.brightness, 0x00)
    } else {
        make_rgb_slot(
            cfg.mode.id(),
            cfg.color,
            cfg.full_spectrum,
            cfg.brightness,
            cfg.speed,
        )
    };

    // Update the matching slot in pages 0..=4 and write it to page 17
    let active_idx = cfg.mode.slot_index();
    let p = active_idx / 4;
    let off = (active_idx % 4) * 16;
    pages[p][off..off + 16].copy_from_slice(&active_slot);

    // Pages 8..=16: per-key LED colors (`80 RR GG BB`) for light_index 0..143
    for page in &mut pages[8..=16] {
        for k in 0..16 {
            page[k * 4] = 0x80;
        }
    }
    for (&light_index, &(r, g, b)) in &cfg.per_key {
        if light_index < MAX_LIGHT_INDEX {
            let page_idx = 8 + (light_index / 16);
            let offset = (light_index % 16) * 4;
            pages[page_idx][offset..offset + 4].copy_from_slice(&[0x80, r, g, b]);
        }
    }

    // Page 17: active mode slot
    pages[17][0..16].copy_from_slice(&active_slot);
    pages
}

// ============================================================================
// Smart Wheel (`04 25`, 1 page = 64 bytes, no `04 f0` apply)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmartWheelSlot {
    pub mode: u8,
    pub color: (u8, u8, u8),
}

pub fn parse_wheel_mode(s: &str) -> Result<u8, String> {
    let norm = s.trim().to_lowercase().replace(['-', ' '], "_");
    match norm.as_str() {
        "volume" | "vol" => Ok(0x00),
        "task_switcher" | "taskswitcher" | "alt_tab" | "alttab" => Ok(0x01),
        "vertical_scroll" | "vscroll" | "scroll" => Ok(0x02),
        "brightness" | "led" | "backlight" => Ok(0x03),
        "horizontal_scroll" | "hscroll" | "horiz_scroll" => Ok(0x04),
        "zoom" => Ok(0x05),
        other => parse_u8_auto(other).map_err(|_| format!("unknown Smart Wheel mode '{s}'")),
    }
}

pub fn wheel_mode_name(mode: u8) -> &'static str {
    match mode {
        0x00 => "volume",
        0x01 => "task_switcher",
        0x02 => "vertical_scroll",
        0x03 => "brightness",
        0x04 => "horizontal_scroll",
        0x05 => "zoom",
        _ => "custom",
    }
}

pub fn baseline_smart_wheel_slots() -> [SmartWheelSlot; 4] {
    [
        SmartWheelSlot { mode: 0x00, color: (0xff, 0x66, 0x00) },
        SmartWheelSlot { mode: 0x01, color: (0xff, 0x20, 0x20) },
        SmartWheelSlot { mode: 0x02, color: (0x4c, 0x20, 0xff) },
        SmartWheelSlot { mode: 0x03, color: (0x20, 0xff, 0x20) },
    ]
}

pub fn build_smart_wheel_page(slots: &[SmartWheelSlot; 4]) -> [u8; 64] {
    let mut page = [0u8; 64];
    for (i, slot) in slots.iter().enumerate() {
        let off = i * 4;
        page[off] = slot.mode;
        page[off + 1] = slot.color.0;
        page[off + 2] = slot.color.1;
        page[off + 3] = slot.color.2;
    }
    page[62] = 0xaa;
    page[63] = 0x55;
    page
}

// ============================================================================
// Color parsing helpers
// ============================================================================

pub fn parse_color(input: &str) -> Result<(u8, u8, u8), String> {
    match input.trim().to_lowercase().as_str() {
        "red" => Ok((0xff, 0x00, 0x00)),
        "green" => Ok((0x00, 0xff, 0x00)),
        "blue" => Ok((0x00, 0x00, 0xff)),
        "white" => Ok((0xff, 0xff, 0xff)),
        "off" | "black" => Ok((0x00, 0x00, 0x00)),
        "yellow" => Ok((0xff, 0xff, 0x00)),
        "cyan" => Ok((0x00, 0xff, 0xff)),
        "magenta" => Ok((0xff, 0x00, 0xff)),
        "orange" => Ok((0xff, 0x66, 0x00)),
        "purple" => Ok((0xa0, 0x20, 0xf0)),
        other => {
            let clean = other.trim_start_matches('#');
            if clean.len() != 6 {
                return Err(format!(
                    "expected named color or 6-digit hex (e.g. ff0000), got '{input}'"
                ));
            }
            let r = u8::from_str_radix(&clean[0..2], 16).map_err(|_| "invalid red hex")?;
            let g = u8::from_str_radix(&clean[2..4], 16).map_err(|_| "invalid green hex")?;
            let b = u8::from_str_radix(&clean[4..6], 16).map_err(|_| "invalid blue hex")?;
            Ok((r, g, b))
        }
    }
}

pub fn format_hex_color((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_to_page(hex: &str) -> [u8; 64] {
        assert_eq!(hex.len(), 128);
        let mut out = [0u8; 64];
        for i in 0..64 {
            out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        out
    }

    #[test]
    fn baseline_base_matches_capture_byte_for_byte() {
        let pages = baseline_base_keymap();
        assert_eq!(get_keymap_slot(&pages, 18), [0x01, 0x03, 0x01, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 36), [0x01, 0x03, 0xff, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 53), [0x01, 0x01, 0x01, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 54), [0x01, 0x01, 0x02, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 71), [0x01, 0x01, 0x04, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 90), [0x02, 0x00, 0x39, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 94), [0x02, 0x00, 0x2a, 0x00]);
        assert_eq!(get_keymap_slot(&pages, 116), [0x00, 0x00, 0x00, 0x00]);
        assert_eq!(&pages[9][62..64], &[0xaa, 0x55]);
    }

    #[test]
    fn baseline_fn_matches_capture_byte_for_byte() {
        // Exact 10 pages of `04 27` from references/captures/filtered_apply_default_profile.txt
        let captured = [
            "00000000000000000383010003cd000003b7000003b6000003b50000050300000503000005030000050300000503000005030000050300000000000000000000",
            "00000000000000000000000005030000050300000503000005030000050300000503000005030000050300000503000005030000050300000503000005030000",
            "05030000050300000503000000000000000000000503000000000000010104000200520001010200010301000503000005030000010102000200520001010100",
            "010301000503000005030000050300000503000000000000000000000503000005030000020050000200510002004f000103ff00020001000200010002005000",
            "0200510002004f000103ff0005030000050300000503000005030000000000000000000005030000050300000503000005030000050300000503000002000100",
            "020050000200510002004f0001010400050300000503000005030000050300000503000000000000000000000503000000000000050300000101010005030000",
            "00000000050300000503000005030000050300000503000005030000050300000503000005030000050300000000000000000000010101000000000000000000",
            "02004700050300000000000002004900050300000503000005030000050300000503000005030000050300000503000000000000000000000000000000000000",
            "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aa55",
        ];
        let built = baseline_fn_keymap();
        for (i, hex) in captured.iter().enumerate() {
            assert_eq!(built[i], hex_to_page(hex), "Fn keymap mismatch on page {i}");
        }
    }

    #[test]
    fn rgb_red_matches_capture_byte_for_byte() {
        let mut per_key = BTreeMap::new();
        // Default WASD white from capture (W=39, A=56, S=57, D=58)
        for idx in [39, 56, 57, 58] {
            per_key.insert(idx, (0xff, 0xff, 0xff));
        }
        let pages = build_rgb_pages(&RgbConfig {
            mode: RgbMode::Static,
            color: (0xff, 0x00, 0x00),
            full_spectrum: false,
            brightness: 6,
            speed: 6,
            per_key,
        });

        assert_eq!(
            pages[0],
            hex_to_page("01ff000000000000000606000000aa550200ffff00000000000606000000aa5503ff000000000000000606000000aa5504ffff0000000000000606000000aa55")
        );
        assert_eq!(
            pages[4],
            hex_to_page("11a020f000000000010606000000aa5512a020f000000000010606000000aa5513a020f000000000010606000000aa558000000000000000000500000000aa55")
        );
        // W = light_index 39 -> page 10, slot 7 -> offset 28
        assert_eq!(&pages[10][28..32], &[0x80, 0xff, 0xff, 0xff]);
        // A, S, D = 56, 57, 58 -> page 11, slots 8, 9, 10
        assert_eq!(&pages[11][32..36], &[0x80, 0xff, 0xff, 0xff]);
        assert_eq!(&pages[11][36..40], &[0x80, 0xff, 0xff, 0xff]);
        assert_eq!(&pages[11][40..44], &[0x80, 0xff, 0xff, 0xff]);
        // Page 17 active slot
        assert_eq!(
            pages[17],
            hex_to_page("01ff000000000000000606000000aa55000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000")
        );
    }

    #[test]
    fn rgb_per_key_sets_mode_80_and_light_indices() {
        let mut per_key = BTreeMap::new();
        per_key.insert(1, (0xff, 0x00, 0x00));   // Esc -> red
        per_key.insert(116, (0x00, 0xff, 0x00)); // Insert -> green
        let pages = build_rgb_pages(&RgbConfig {
            mode: RgbMode::UserDefine,
            color: (0, 0, 0),
            full_spectrum: false,
            brightness: 5,
            speed: 0,
            per_key,
        });
        assert_eq!(&pages[17][0..16], &make_rgb_slot(0x80, (0, 0, 0), false, 5, 0));
        // Esc = 1 -> page 8, offset 4
        assert_eq!(&pages[8][4..8], &[0x80, 0xff, 0x00, 0x00]);
        // Insert = 116 -> page 8 + 7 = 15, offset (116 % 16) * 4 = 16
        assert_eq!(&pages[15][16..20], &[0x80, 0x00, 0xff, 0x00]);
    }

    #[test]
    fn smart_wheel_matches_capture_byte_for_byte() {
        let page = build_smart_wheel_page(&baseline_smart_wheel_slots());
        assert_eq!(
            page,
            hex_to_page("00ff660001ff2020024c20ff0320ff2000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000aa55")
        );
    }

    #[test]
    fn key_entry_parser_handles_all_formats() {
        assert_eq!(KeyEntry::parse("ctrl+c").unwrap().to_bytes(), [0x02, 0x01, 0x06, 0x00]);
        assert_eq!(KeyEntry::parse("lctrl+lshift+a").unwrap().to_bytes(), [0x02, 0x03, 0x04, 0x00]);
        assert_eq!(KeyEntry::parse("scroll_up").unwrap().to_bytes(), [0x01, 0x03, 0x01, 0x00]);
        assert_eq!(KeyEntry::parse("scroll_down").unwrap().to_bytes(), [0x01, 0x03, 0xff, 0x00]);
        assert_eq!(KeyEntry::parse("mouse_left").unwrap().to_bytes(), [0x01, 0x01, 0x01, 0x00]);
        assert_eq!(KeyEntry::parse("play_pause").unwrap().to_bytes(), [0x03, 0xcd, 0x00, 0x00]);
        assert_eq!(KeyEntry::parse("macro:5").unwrap().to_bytes(), [0x05, 0x02, 0x05, 0x00]);
        assert_eq!(KeyEntry::parse("disabled").unwrap().to_bytes(), [0x05, 0x03, 0x00, 0x00]);
        assert_eq!(KeyEntry::parse("raw:02000100").unwrap().to_bytes(), [0x02, 0x00, 0x01, 0x00]);
    }
}
