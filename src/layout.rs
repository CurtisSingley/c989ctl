#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyInfo {
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub key_index: usize,
    pub light_index: usize,
    pub default_code: u8,
    pub remappable: bool,
}

const fn key(
    name: &'static str,
    aliases: &'static [&'static str],
    index: usize,
    default_code: u8,
) -> KeyInfo {
    KeyInfo {
        name,
        aliases,
        key_index: index,
        light_index: index,
        default_code,
        remappable: true,
    }
}

const fn locked_key(
    name: &'static str,
    aliases: &'static [&'static str],
    index: usize,
    default_code: u8,
) -> KeyInfo {
    KeyInfo {
        name,
        aliases,
        key_index: index,
        light_index: index,
        default_code,
        remappable: false,
    }
}

/// Complete physical key map for Cloud Nine C989 ErgoFSK (`<LayoutK89M>` in KeyboardLayout.xml).
///
/// The 10 left-side macro keys use their physical keycap labels:
/// - Column 1 (top to bottom): `m1` (90), `m2` (18), `m3` (36), `m4` (54), `m5` (72, locked)
/// - Column 2 (top to bottom): `m6` (17), `m7` (35), `m8` (53), `m9` (71), `m10` (89, locked)
pub const KEYS: &[KeyInfo] = &[
    // Function row
    key("esc", &["escape"], 1, 0x29),
    key("f1", &[], 2, 0x3a),
    key("f2", &[], 3, 0x3b),
    key("f3", &[], 4, 0x3c),
    key("f4", &[], 5, 0x3d),
    key("f5", &[], 6, 0x3e),
    key("f6", &[], 7, 0x3f),
    key("f7", &[], 8, 0x40),
    key("f8", &[], 9, 0x41),
    key("f9", &[], 10, 0x42),
    key("f10", &[], 11, 0x43),
    key("f11", &[], 12, 0x44),
    key("f12", &[], 13, 0x45),
    key("prtsc", &["print_screen", "printscreen", "sysrq"], 112, 0x46),
    key("scrlk", &["scroll_lock", "scrolllock"], 113, 0x47),
    key("pause", &["break"], 115, 0x48),

    // Number row + left macro row 1 + nav + numpad row 1
    key("m1", &["select_all", "selectall", "select-all"], 90, 0x81),
    key("m6", &[], 17, 0x86),
    key("grave", &["~", "`", "tilde", "backtick"], 19, 0x35),
    key("1", &[], 20, 0x1e),
    key("2", &[], 21, 0x1f),
    key("3", &[], 22, 0x20),
    key("4", &[], 23, 0x21),
    key("5", &[], 24, 0x22),
    key("6", &[], 25, 0x23),
    key("7", &[], 26, 0x24),
    key("8", &[], 27, 0x25),
    key("9", &[], 28, 0x26),
    key("0", &[], 29, 0x27),
    key("minus", &["-", "dash"], 30, 0x2d),
    key("equal", &["=", "equals", "plus"], 31, 0x2e),
    key("backspace", &["bks", "bs", "bksp"], 103, 0x2a),
    key("insert", &["ins"], 116, 0x49),
    key("home", &[], 117, 0x4a),
    key("pgup", &["pageup", "page_up", "page-up"], 118, 0x4b),
    key("numlock", &["num", "num_lock"], 32, 0x53),
    key("kp_slash", &["kp/", "num_slash", "kp_divide"], 33, 0x54),
    key("kp_asterisk", &["kp*", "kp_star", "kp_multiply", "num_asterisk"], 34, 0x55),
    key("kp_minus", &["kp-", "num_minus", "kp_subtract"], 122, 0x56),

    // QWERTY row + left macro row 2 + nav + numpad row 2
    key("m2", &["copy"], 18, 0x82),
    key("m7", &[], 35, 0x87),
    key("tab", &[], 37, 0x2b),
    key("q", &[], 38, 0x14),
    key("w", &[], 39, 0x1a),
    key("e", &[], 40, 0x08),
    key("r", &[], 41, 0x15),
    key("t", &[], 42, 0x17),
    key("y", &[], 43, 0x1c),
    key("u", &[], 44, 0x18),
    key("i", &[], 45, 0x0c),
    key("o", &[], 46, 0x12),
    key("p", &[], 47, 0x13),
    key("lbracket", &["[", "left_bracket"], 48, 0x2f),
    key("rbracket", &["]", "right_bracket"], 49, 0x30),
    key("backslash", &["\\", "\\|", "pipe"], 67, 0x31),
    key("del", &["delete"], 119, 0x4c),
    key("end", &[], 120, 0x4d),
    key("pgdn", &["pagedown", "page_down", "page-down"], 121, 0x4e),
    key("kp_7", &["kp7", "num7"], 50, 0x5f),
    key("kp_8", &["kp8", "num8"], 51, 0x60),
    key("kp_9", &["kp9", "num9"], 52, 0x61),

    // Home row + left macro row 3 + numpad row 3
    key("m3", &["paste"], 36, 0x83),
    key("m8", &[], 53, 0x88),
    key("caps", &["capslock", "caps_lock", "caps-lock"], 55, 0x39),
    key("a", &[], 56, 0x04),
    key("s", &[], 57, 0x16),
    key("d", &[], 58, 0x07),
    key("f", &[], 59, 0x09),
    key("g", &[], 60, 0x0a),
    key("h", &[], 61, 0x0b),
    key("j", &[], 62, 0x0d),
    key("k", &[], 63, 0x0e),
    key("l", &[], 64, 0x0f),
    key("semicolon", &[";"], 65, 0x33),
    key("quote", &["'", "apostrophe"], 66, 0x34),
    key("enter", &["return", "ret"], 85, 0x28),
    key("kp_4", &["kp4", "num4"], 68, 0x5c),
    key("kp_5", &["kp5", "num5"], 69, 0x5d),
    key("kp_6", &["kp6", "num6"], 70, 0x5e),
    key("kp_plus", &["kp+", "num_plus", "kp_add"], 123, 0x57),

    // Bottom alpha row + left macro row 4 + up arrow + numpad row 4
    key("m4", &["cut"], 54, 0x84),
    key("m9", &[], 71, 0x89),
    key("lshift", &["left_shift", "left-shift"], 73, 0xe1),
    key("z", &[], 74, 0x1d),
    key("x", &[], 75, 0x1b),
    key("c", &[], 76, 0x06),
    key("v", &[], 77, 0x19),
    key("b", &[], 78, 0x05),
    key("n", &[], 79, 0x11),
    key("m", &[], 80, 0x10),
    key("comma", &[",", "<"], 81, 0x36),
    key("period", &[".", ">", "dot"], 82, 0x37),
    key("slash", &["/", "?"], 83, 0x38),
    key("rshift", &["right_shift", "right-shift"], 84, 0xe5),
    key("up", &["arrow_up"], 101, 0x52),
    key("kp_1", &["kp1", "num1"], 86, 0x59),
    key("kp_2", &["kp2", "num2"], 87, 0x5a),
    key("kp_3", &["kp3", "num3"], 88, 0x5b),

    // Bottom row
    locked_key("m5", &["lfn", "left_fn", "left-fn"], 72, 0x85),
    locked_key("m10", &["light", "led", "backlight"], 89, 0x8a),
    key("lctrl", &["left_ctrl", "left-ctrl"], 91, 0xe0),
    key("lwin", &["win", "super", "lsuper", "lgui", "meta"], 92, 0xe3),
    key("lalt", &["left_alt", "left-alt"], 93, 0xe2),
    key("left_space", &["left-space", "lspace"], 94, 0x2c),
    key("right_space", &["right-space", "rspace"], 109, 0x8b),
    key("ralt", &["right_alt", "right-alt", "altgr"], 95, 0xe6),
    locked_key("rfn", &["fn", "right_fn", "right-fn"], 96, 0xaf),
    key("app", &["menu", "context"], 97, 0x65),
    key("rctrl", &["right_ctrl", "right-ctrl"], 98, 0xe4),
    key("left", &["arrow_left"], 99, 0x50),
    key("down", &["arrow_down"], 100, 0x51),
    key("right", &["arrow_right"], 102, 0x4f),
    key("kp_0", &["kp0", "num0"], 104, 0x62),
    key("kp_dot", &["kp.", "kp_period", "kp_del", "num_dot"], 105, 0x63),
    key("kp_enter", &["kpenter", "num_enter"], 106, 0x58),
];

/// Physical key indices that remain `00 00 00 00` (firmware default) in the Fn layer (`04 27`)
/// rather than `05 03 00 00` (disabled), matching the official Windows utility captures.
pub const FN_FIRMWARE_DEFAULT_INDICES: &[usize] = &[
    1,  // Esc
    17, // M1
    18, // Copy
    35, // M2
    36, // Paste
    38, // Q
    53, // M3
    54, // Cut
    71, // M4
    72, // Left Fn
    89, // Light
    90, // Select all
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

/// Resolve a key name to its standard USB HID Keyboard/Keypad Usage ID (Usage Page 0x07).
pub fn hid_usage_by_name(name: &str) -> Option<u8> {
    let q = name.trim().to_lowercase();
    let normalized = q.replace(['-', ' '], "_");

    // Special output-only names not tied to a single physical key name
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

    // Look up in physical layout table, rejecting vendor-specific pseudo-codes (0x80..0xaf)
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
