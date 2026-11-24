pub const LAYOUT_WIDTH: f32 = 1360.0;
pub const LAYOUT_HEIGHT: f32 = 340.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeyInfo {
    pub name: &'static str,
    pub label: &'static str,
    pub aliases: &'static [&'static str],
    pub key_index: usize,
    pub light_index: usize,
    pub default_code: u8,
    pub remappable: bool,
    /// Visual bounding box `(x, y, width, height)` in `<LayoutK89M>` coordinates (1360×340).
    pub rect: (f32, f32, f32, f32),
}

const fn key(
    name: &'static str,
    label: &'static str,
    aliases: &'static [&'static str],
    index: usize,
    default_code: u8,
    rect: (f32, f32, f32, f32),
) -> KeyInfo {
    KeyInfo {
        name,
        label,
        aliases,
        key_index: index,
        light_index: index,
        default_code,
        remappable: true,
        rect,
    }
}

const fn locked_key(
    name: &'static str,
    label: &'static str,
    aliases: &'static [&'static str],
    index: usize,
    default_code: u8,
    rect: (f32, f32, f32, f32),
) -> KeyInfo {
    KeyInfo {
        name,
        label,
        aliases,
        key_index: index,
        light_index: index,
        default_code,
        remappable: false,
        rect,
    }
}

/// Physical key map for Cloud Nine C989 ErgoFSK (`<LayoutK89M>`).
/// Left macro columns use physical keycap labels: M1..M5 (col 1), M6..M10 (col 2).
pub const KEYS: &[KeyInfo] = &[
    key("esc", "Esc", &["escape"], 1, 0x29, (122.0, 1.0, 50.0, 50.0)),
    key("f1", "F1", &[], 2, 0x3a, (186.0, 1.0, 50.0, 50.0)),
    key("f2", "F2", &[], 3, 0x3b, (238.0, 1.0, 50.0, 50.0)),
    key("f3", "F3", &[], 4, 0x3c, (290.0, 1.0, 50.0, 50.0)),
    key("f4", "F4", &[], 5, 0x3d, (342.0, 1.0, 50.0, 50.0)),
    key("f5", "F5", &[], 6, 0x3e, (405.0, 1.0, 50.0, 50.0)),
    key("f6", "F6", &[], 7, 0x3f, (570.0, 1.0, 50.0, 50.0)),
    key("f7", "F7", &[], 8, 0x40, (622.0, 1.0, 50.0, 50.0)),
    key("f8", "F8", &[], 9, 0x41, (674.0, 1.0, 50.0, 50.0)),
    key("f9", "F9", &[], 10, 0x42, (750.0, 1.0, 50.0, 50.0)),
    key("f10", "F10", &[], 11, 0x43, (802.0, 1.0, 50.0, 50.0)),
    key("f11", "F11", &[], 12, 0x44, (854.0, 1.0, 50.0, 50.0)),
    key("f12", "F12", &[], 13, 0x45, (906.0, 1.0, 50.0, 50.0)),
    key("prtsc", "PrtSc", &["print_screen", "printscreen", "sysrq"], 112, 0x46, (975.0, 1.0, 50.0, 50.0)),
    key("scrlk", "ScrLk", &["scroll_lock", "scrolllock"], 113, 0x47, (1027.0, 1.0, 50.0, 50.0)),
    key("pause", "Pause", &["break"], 115, 0x48, (1079.0, 1.0, 50.0, 50.0)),

    key("m1", "M1", &["select_all", "selectall", "select-all"], 90, 0x81, (1.0, 78.0, 50.0, 50.0)),
    key("m6", "M6", &[], 17, 0x86, (53.0, 78.0, 50.0, 50.0)),
    key("grave", "~", &["~", "`", "tilde", "backtick"], 19, 0x35, (122.0, 78.0, 50.0, 50.0)),
    key("1", "1", &[], 20, 0x1e, (174.0, 78.0, 50.0, 50.0)),
    key("2", "2", &[], 21, 0x1f, (226.0, 78.0, 50.0, 50.0)),
    key("3", "3", &[], 22, 0x20, (278.0, 78.0, 50.0, 50.0)),
    key("4", "4", &[], 23, 0x21, (330.0, 78.0, 50.0, 50.0)),
    key("5", "5", &[], 24, 0x22, (382.0, 78.0, 50.0, 50.0)),
    key("6", "6", &[], 25, 0x23, (434.0, 78.0, 50.0, 50.0)),
    key("7", "7", &[], 26, 0x24, (543.0, 78.0, 50.0, 50.0)),
    key("8", "8", &[], 27, 0x25, (595.0, 78.0, 50.0, 50.0)),
    key("9", "9", &[], 28, 0x26, (647.0, 78.0, 50.0, 50.0)),
    key("0", "0", &[], 29, 0x27, (699.0, 78.0, 50.0, 50.0)),
    key("minus", "-", &["-", "dash"], 30, 0x2d, (751.0, 78.0, 50.0, 50.0)),
    key("equal", "=", &["=", "equals", "plus"], 31, 0x2e, (803.0, 78.0, 50.0, 50.0)),
    key("backspace", "BKS", &["bks", "bs", "bksp"], 103, 0x2a, (855.0, 78.0, 101.0, 50.0)),
    key("insert", "Ins", &["ins"], 116, 0x49, (975.0, 78.0, 50.0, 50.0)),
    key("home", "Home", &[], 117, 0x4a, (1027.0, 78.0, 50.0, 50.0)),
    key("pgup", "PgUp", &["pageup", "page_up", "page-up"], 118, 0x4b, (1079.0, 78.0, 50.0, 50.0)),
    key("numlock", "Num", &["num", "num_lock"], 32, 0x53, (1150.0, 78.0, 50.0, 50.0)),
    key("kp_slash", "/", &["kp/", "num_slash", "kp_divide"], 33, 0x54, (1202.0, 78.0, 50.0, 50.0)),
    key("kp_asterisk", "*", &["kp*", "kp_star", "kp_multiply", "num_asterisk"], 34, 0x55, (1254.0, 78.0, 50.0, 50.0)),
    key("kp_minus", "-", &["kp-", "num_minus", "kp_subtract"], 122, 0x56, (1306.0, 78.0, 50.0, 50.0)),

    key("m2", "M2", &["copy"], 18, 0x82, (1.0, 130.0, 50.0, 50.0)),
    key("m7", "M7", &[], 35, 0x87, (53.0, 130.0, 50.0, 50.0)),
    key("tab", "Tab", &[], 37, 0x2b, (122.0, 130.0, 75.0, 50.0)),
    key("q", "Q", &[], 38, 0x14, (199.0, 130.0, 50.0, 50.0)),
    key("w", "W", &[], 39, 0x1a, (251.0, 130.0, 50.0, 50.0)),
    key("e", "E", &[], 40, 0x08, (303.0, 130.0, 50.0, 50.0)),
    key("r", "R", &[], 41, 0x15, (355.0, 130.0, 50.0, 50.0)),
    key("t", "T", &[], 42, 0x17, (407.0, 130.0, 50.0, 50.0)),
    key("y", "Y", &[], 43, 0x1c, (519.0, 130.0, 50.0, 50.0)),
    key("u", "U", &[], 44, 0x18, (571.0, 130.0, 50.0, 50.0)),
    key("i", "I", &[], 45, 0x0c, (623.0, 130.0, 50.0, 50.0)),
    key("o", "O", &[], 46, 0x12, (675.0, 130.0, 50.0, 50.0)),
    key("p", "P", &[], 47, 0x13, (727.0, 130.0, 50.0, 50.0)),
    key("lbracket", "[", &["[", "left_bracket"], 48, 0x2f, (779.0, 130.0, 50.0, 50.0)),
    key("rbracket", "]", &["]", "right_bracket"], 49, 0x30, (831.0, 130.0, 50.0, 50.0)),
    key("backslash", "\\|", &["\\", "\\|", "pipe"], 67, 0x31, (883.0, 130.0, 73.0, 50.0)),
    key("del", "Del", &["delete"], 119, 0x4c, (975.0, 130.0, 50.0, 50.0)),
    key("end", "End", &[], 120, 0x4d, (1027.0, 130.0, 50.0, 50.0)),
    key("pgdn", "PgDn", &["pagedown", "page_down", "page-down"], 121, 0x4e, (1079.0, 130.0, 50.0, 50.0)),
    key("kp_7", "7", &["kp7", "num7"], 50, 0x5f, (1150.0, 130.0, 50.0, 50.0)),
    key("kp_8", "8", &["kp8", "num8"], 51, 0x60, (1202.0, 130.0, 50.0, 50.0)),
    key("kp_9", "9", &["kp9", "num9"], 52, 0x61, (1254.0, 130.0, 50.0, 50.0)),

    key("m3", "M3", &["paste"], 36, 0x83, (1.0, 182.0, 50.0, 50.0)),
    key("m8", "M8", &[], 53, 0x88, (53.0, 182.0, 50.0, 50.0)),
    key("caps", "Caps", &["capslock", "caps_lock", "caps-lock"], 55, 0x39, (122.0, 182.0, 87.0, 50.0)),
    key("a", "A", &[], 56, 0x04, (211.0, 182.0, 50.0, 50.0)),
    key("s", "S", &[], 57, 0x16, (263.0, 182.0, 50.0, 50.0)),
    key("d", "D", &[], 58, 0x07, (315.0, 182.0, 50.0, 50.0)),
    key("f", "F", &[], 59, 0x09, (367.0, 182.0, 50.0, 50.0)),
    key("g", "G", &[], 60, 0x0a, (419.0, 182.0, 50.0, 50.0)),
    key("h", "H", &[], 61, 0x0b, (532.0, 182.0, 50.0, 50.0)),
    key("j", "J", &[], 62, 0x0d, (584.0, 182.0, 50.0, 50.0)),
    key("k", "K", &[], 63, 0x0e, (636.0, 182.0, 50.0, 50.0)),
    key("l", "L", &[], 64, 0x0f, (688.0, 182.0, 50.0, 50.0)),
    key("semicolon", ";", &[";"], 65, 0x33, (740.0, 182.0, 50.0, 50.0)),
    key("quote", "'", &["'", "apostrophe"], 66, 0x34, (792.0, 182.0, 50.0, 50.0)),
    key("enter", "Enter", &["return", "ret"], 85, 0x28, (844.0, 182.0, 112.0, 50.0)),
    key("kp_4", "4", &["kp4", "num4"], 68, 0x5c, (1150.0, 182.0, 50.0, 50.0)),
    key("kp_5", "5", &["kp5", "num5"], 69, 0x5d, (1202.0, 182.0, 50.0, 50.0)),
    key("kp_6", "6", &["kp6", "num6"], 70, 0x5e, (1254.0, 182.0, 50.0, 50.0)),
    key("kp_plus", "+", &["kp+", "num_plus", "kp_add"], 123, 0x57, (1306.0, 130.0, 50.0, 102.0)),

    key("m4", "M4", &["cut"], 54, 0x84, (1.0, 234.0, 50.0, 50.0)),
    key("m9", "M9", &[], 71, 0x89, (53.0, 234.0, 50.0, 50.0)),
    key("lshift", "Shift", &["left_shift", "left-shift"], 73, 0xe1, (122.0, 234.0, 113.0, 50.0)),
    key("z", "Z", &[], 74, 0x1d, (237.0, 234.0, 50.0, 50.0)),
    key("x", "X", &[], 75, 0x1b, (289.0, 234.0, 50.0, 50.0)),
    key("c", "C", &[], 76, 0x06, (341.0, 234.0, 50.0, 50.0)),
    key("v", "V", &[], 77, 0x19, (393.0, 234.0, 50.0, 50.0)),
    key("b", "B", &[], 78, 0x05, (445.0, 234.0, 50.0, 50.0)),
    key("n", "N", &[], 79, 0x11, (557.0, 234.0, 50.0, 50.0)),
    key("m", "M", &[], 80, 0x10, (609.0, 234.0, 50.0, 50.0)),
    key("comma", ",", &[",", "<"], 81, 0x36, (661.0, 234.0, 50.0, 50.0)),
    key("period", ".", &[".", ">", "dot"], 82, 0x37, (713.0, 234.0, 50.0, 50.0)),
    key("slash", "/", &["/", "?"], 83, 0x38, (765.0, 234.0, 50.0, 50.0)),
    key("rshift", "Shift", &["right_shift", "right-shift"], 84, 0xe5, (817.0, 234.0, 139.0, 50.0)),
    key("up", "Up", &["arrow_up"], 101, 0x52, (1027.0, 234.0, 50.0, 50.0)),
    key("kp_1", "1", &["kp1", "num1"], 86, 0x59, (1150.0, 234.0, 50.0, 50.0)),
    key("kp_2", "2", &["kp2", "num2"], 87, 0x5a, (1202.0, 234.0, 50.0, 50.0)),
    key("kp_3", "3", &["kp3", "num3"], 88, 0x5b, (1254.0, 234.0, 50.0, 50.0)),

    locked_key("m5", "M5/Fn", &["lfn", "left_fn", "left-fn"], 72, 0x85, (1.0, 286.0, 50.0, 50.0)),
    locked_key("m10", "M10", &["light", "led", "backlight"], 89, 0x8a, (53.0, 286.0, 50.0, 50.0)),
    key("lctrl", "Ctrl", &["left_ctrl", "left-ctrl"], 91, 0xe0, (122.0, 286.0, 61.0, 50.0)),
    key("lwin", "Win", &["win", "super", "lsuper", "lgui", "meta"], 92, 0xe3, (185.0, 286.0, 61.0, 50.0)),
    key("lalt", "Alt", &["left_alt", "left-alt"], 93, 0xe2, (248.0, 286.0, 61.0, 50.0)),
    key("left_space", "Left Space", &["left-space", "lspace"], 94, 0x2c, (311.0, 286.0, 183.0, 50.0)),
    key("right_space", "Right Space", &["right-space", "rspace"], 109, 0x8b, (519.0, 286.0, 185.0, 50.0)),
    key("ralt", "Alt", &["right_alt", "right-alt", "altgr"], 95, 0xe6, (706.0, 286.0, 61.0, 50.0)),
    locked_key("rfn", "Fn", &["fn", "right_fn", "right-fn"], 96, 0xaf, (769.0, 286.0, 61.0, 50.0)),
    key("app", "App", &["menu", "context"], 97, 0x65, (832.0, 286.0, 61.0, 50.0)),
    key("rctrl", "Ctrl", &["right_ctrl", "right-ctrl"], 98, 0xe4, (895.0, 286.0, 61.0, 50.0)),
    key("left", "Left", &["arrow_left"], 99, 0x50, (975.0, 286.0, 50.0, 50.0)),
    key("down", "Down", &["arrow_down"], 100, 0x51, (1027.0, 286.0, 50.0, 50.0)),
    key("right", "Right", &["arrow_right"], 102, 0x4f, (1079.0, 286.0, 50.0, 50.0)),
    key("kp_0", "0", &["kp0", "num0"], 104, 0x62, (1150.0, 286.0, 102.0, 50.0)),
    key("kp_dot", ".", &["kp.", "kp_period", "kp_del", "num_dot"], 105, 0x63, (1254.0, 286.0, 50.0, 50.0)),
    key("kp_enter", "Enter", &["kpenter", "num_enter"], 106, 0x58, (1306.0, 234.0, 50.0, 102.0)),
];

pub const ZONE_WASD: &[&str] = &["w", "a", "s", "d"];
pub const ZONE_NUMBERS: &[&str] = &["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"];
pub const ZONE_MACROS: &[&str] = &["m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9", "m10"];
pub const ZONE_ARROWS: &[&str] = &["up", "down", "left", "right"];
pub const ZONE_NUMPAD: &[&str] = &[
    "numlock", "kp_slash", "kp_asterisk", "kp_minus", "kp_7", "kp_8", "kp_9", "kp_plus",
    "kp_4", "kp_5", "kp_6", "kp_1", "kp_2", "kp_3", "kp_0", "kp_dot", "kp_enter",
];

/// Physical key indices that remain `00 00 00 00` (firmware default) in the Fn layer (`04 27`)
/// rather than `05 03 00 00` (disabled), matching the official Windows utility captures.
pub const FN_FIRMWARE_DEFAULT_INDICES: &[usize] = &[
    1,  // Esc
    17, // M6
    18, // M2
    35, // M7
    36, // M3
    38, // Q
    53, // M8
    54, // M4
    71, // M9
    72, // M5 / Left Fn
    89, // M10 / Light
    90, // M1
    92, // Win
    96, // Right Fn
];

pub fn find_key(query: &str) -> Option<&'static KeyInfo> {
    let q = query.trim().to_lowercase();
    if let Some(rest) = q.strip_prefix('#') {
        if let Ok(idx) = rest.parse::<usize>() {
            return key_by_index(idx);
        }
    }
    let normalized = q.replace(' ', "_");
    KEYS.iter().find(|k| {
        k.name == normalized
            || k.name == q
            || k.aliases.iter().any(|a| *a == q || *a == normalized)
    })
}

pub fn key_by_index(index: usize) -> Option<&'static KeyInfo> {
    KEYS.iter().find(|k| k.key_index == index)
}

pub fn firmware_default_hint(name: &str) -> Option<&'static str> {
    match name {
        "m1" => Some("Ctrl+A"),
        "m2" => Some("Ctrl+C"),
        "m3" => Some("Ctrl+V"),
        "m4" => Some("Ctrl+X"),
        "m6" => Some("Ctrl+1"),
        "m7" => Some("Ctrl+2"),
        "m8" => Some("Ctrl+3"),
        "m9" => Some("Ctrl+4"),
        _ => None,
    }
}

/// Resolve a key name to its standard USB HID Keyboard/Keypad Usage ID (Usage Page 0x07).
pub fn hid_usage_by_name(name: &str) -> Option<u8> {
    let q = name.trim().to_lowercase();
    let normalized = q.replace(['-', ' '], "_");

    match normalized.as_str() {
        "space" | "spc" => return Some(0x2c),
        "rwin" | "rsuper" | "rgui" | "right_win" => return Some(0xe7),
        "ctrl" => return Some(0xe0),
        "shift" => return Some(0xe1),
        "alt" => return Some(0xe2),
        _ => {}
    }

    if let Some(hex) = normalized.strip_prefix("0x") {
        if let Ok(code) = u8::from_str_radix(hex, 16) {
            return Some(code);
        }
    }

    let key = find_key(&q)?;
    if (0x80..=0xdf).contains(&key.default_code) {
        None
    } else {
        Some(key.default_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn layout_has_115_unique_keys() {
        assert_eq!(KEYS.len(), 115);
        let mut names = HashSet::new();
        let mut indices = HashSet::new();
        for k in KEYS {
            assert!(names.insert(k.name), "Duplicate key name: {}", k.name);
            assert!(
                indices.insert(k.key_index),
                "Duplicate key_index: {}",
                k.key_index
            );
            assert_eq!(k.key_index, k.light_index);
            assert!(k.key_index < 160);
            assert!(k.light_index < 144);
            assert!(k.rect.2 > 0.0 && k.rect.3 > 0.0);
        }
    }

    #[test]
    fn lookup_and_hid_codes_work() {
        assert_eq!(find_key("Insert").unwrap().key_index, 116);
        assert_eq!(find_key("Left Space").unwrap().key_index, 94);
        assert_eq!(find_key("bks").unwrap().key_index, 103);
        assert_eq!(find_key("#116").unwrap().name, "insert");
        assert_eq!(find_key("m1").unwrap().key_index, 90);
        assert_eq!(find_key("m6").unwrap().key_index, 17);
        assert!(!find_key("m5").unwrap().remappable);
        assert!(!find_key("m10").unwrap().remappable);
        assert!(!find_key("rfn").unwrap().remappable);
        assert_eq!(hid_usage_by_name("a"), Some(0x04));
        assert_eq!(hid_usage_by_name("backspace"), Some(0x2a));
        assert_eq!(hid_usage_by_name("caps"), Some(0x39));
        assert_eq!(hid_usage_by_name("space"), Some(0x2c));
        assert_eq!(hid_usage_by_name("m1"), None);
    }
}
