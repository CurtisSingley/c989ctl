use crate::layout::find_key;
use crate::protocol::{
    KEYMAP_PAGES, KeyEntry, RGB_PAGES, RgbConfig, RgbMode, SmartWheelSlot,
    baseline_smart_wheel_slots, build_base_keymap, build_fn_keymap, build_rgb_pages,
    build_smart_wheel_page, format_hex_color, parse_color, parse_wheel_mode, wheel_mode_name,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub rgb: Option<RgbProfile>,
    #[serde(default)]
    pub smart_wheel: Option<Vec<WheelSlotProfile>>,
    #[serde(default)]
    pub keymap: Option<KeymapProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbProfile {
    pub mode: String,
    #[serde(default = "default_color_str")]
    pub color: String,
    #[serde(default = "default_brightness")]
    pub brightness: u8,
    #[serde(default = "default_speed")]
    pub speed: u8,
    #[serde(default)]
    pub full_spectrum: Option<bool>,
    #[serde(default)]
    pub per_key: BTreeMap<String, String>,
}

fn default_color_str() -> String {
    "#0000ff".to_string()
}
fn default_brightness() -> u8 {
    6
}
fn default_speed() -> u8 {
    6
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelSlotProfile {
    pub mode: String,
    pub color: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeymapProfile {
    #[serde(default)]
    pub base: Option<BTreeMap<String, String>>,
    #[serde(default, rename = "fn")]
    pub fn_layer: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct CompiledProfile {
    pub base_keymap: Option<[[u8; 64]; KEYMAP_PAGES]>,
    pub fn_keymap: Option<[[u8; 64]; KEYMAP_PAGES]>,
    pub rgb: Option<[[u8; 64]; RGB_PAGES]>,
    pub smart_wheel: Option<[u8; 64]>,
}

impl Profile {
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|e| format!("failed to read profile '{}': {e}", path.display()))?;
        toml::from_str(&text)
            .map_err(|e| format!("failed to parse TOML profile '{}': {e}", path.display()))
    }

    pub fn to_toml_string(&self) -> Result<String, String> {
        toml::to_string_pretty(self).map_err(|e| format!("failed to serialize profile: {e}"))
    }

    /// Build a Profile matching the user's exact captured daily default profile
    /// (`filtered_apply_default_profile.txt`).
    pub fn baseline() -> Self {
        let mut base = BTreeMap::new();
        base.insert("m1".into(), "caps".into());
        base.insert("m2".into(), "scroll_up".into());
        base.insert("m3".into(), "scroll_down".into());
        base.insert("m4".into(), "mouse_right".into());
        base.insert("m8".into(), "mouse_left".into());
        base.insert("m9".into(), "mouse_middle".into());
        base.insert("left_space".into(), "backspace".into());

        let mut fn_layer = BTreeMap::new();
        fn_layer.insert("f1".into(), "media_player".into());
        fn_layer.insert("f2".into(), "play_pause".into());
        fn_layer.insert("f3".into(), "stop".into());
        fn_layer.insert("f4".into(), "prev_track".into());
        fn_layer.insert("f5".into(), "next_track".into());
        fn_layer.insert("w".into(), "mouse_middle".into());
        fn_layer.insert("e".into(), "up".into());
        fn_layer.insert("r".into(), "mouse_right".into());
        fn_layer.insert("t".into(), "scroll_up".into());
        fn_layer.insert("i".into(), "mouse_right".into());
        fn_layer.insert("o".into(), "up".into());
        fn_layer.insert("p".into(), "mouse_left".into());
        fn_layer.insert("lbracket".into(), "scroll_up".into());
        fn_layer.insert("s".into(), "left".into());
        fn_layer.insert("d".into(), "down".into());
        fn_layer.insert("f".into(), "right".into());
        fn_layer.insert("g".into(), "scroll_down".into());
        fn_layer.insert("h".into(), "raw:02000100".into());
        fn_layer.insert("j".into(), "raw:02000100".into());
        fn_layer.insert("k".into(), "left".into());
        fn_layer.insert("l".into(), "down".into());
        fn_layer.insert("semicolon".into(), "right".into());
        fn_layer.insert("quote".into(), "scroll_down".into());
        fn_layer.insert("n".into(), "raw:02000100".into());
        fn_layer.insert("m".into(), "left".into());
        fn_layer.insert("comma".into(), "down".into());
        fn_layer.insert("period".into(), "right".into());
        fn_layer.insert("slash".into(), "mouse_middle".into());
        fn_layer.insert("left_space".into(), "mouse_left".into());
        fn_layer.insert("right_space".into(), "mouse_left".into());
        fn_layer.insert("prtsc".into(), "scrlk".into());
        fn_layer.insert("pause".into(), "insert".into());

        let mut per_key = BTreeMap::new();
        for k in ["w", "a", "s", "d"] {
            per_key.insert(k.into(), "#ffffff".into());
        }

        let wheel = baseline_smart_wheel_slots()
            .into_iter()
            .map(|s| WheelSlotProfile {
                mode: wheel_mode_name(s.mode).to_string(),
                color: format_hex_color(s.color),
            })
            .collect();

        Self {
            name: "default".into(),
            rgb: Some(RgbProfile {
                mode: "static".into(),
                color: "#0000ff".into(),
                brightness: 5,
                speed: 6,
                full_spectrum: Some(false),
                per_key,
            }),
            smart_wheel: Some(wheel),
            keymap: Some(KeymapProfile {
                base: Some(base),
                fn_layer: Some(fn_layer),
            }),
        }
    }

    pub fn compile(&self) -> Result<CompiledProfile, String> {
        let base_keymap = match self.keymap.as_ref().and_then(|k| k.base.as_ref()) {
            Some(map) => {
                let entries = compile_keymap_entries("keymap.base", map)?;
                Some(build_base_keymap(&entries))
            }
            None => None,
        };

        let fn_keymap = match self.keymap.as_ref().and_then(|k| k.fn_layer.as_ref()) {
            Some(map) => {
                let entries = compile_keymap_entries("keymap.fn", map)?;
                Some(build_fn_keymap(&entries))
            }
            None => None,
        };

        let rgb = match &self.rgb {
            Some(rgb_prof) => {
                let mode = RgbMode::parse(&rgb_prof.mode)?;
                let color = parse_color(&rgb_prof.color)?;
                let full_spectrum = rgb_prof
                    .full_spectrum
                    .unwrap_or_else(|| mode.default_full_spectrum());
                let mut per_key = BTreeMap::new();
                for (key_name, color_str) in &rgb_prof.per_key {
                    let info = find_key(key_name).ok_or_else(|| {
                        format!("rgb.per_key: unknown physical key '{key_name}'")
                    })?;
                    let c = parse_color(color_str).map_err(|e| {
                        format!("rgb.per_key.{key_name}: {e}")
                    })?;
                    per_key.insert(info.light_index, c);
                }
                let cfg = RgbConfig {
                    mode,
                    color,
                    full_spectrum,
                    brightness: rgb_prof.brightness,
                    speed: rgb_prof.speed,
                    per_key,
                };
                Some(build_rgb_pages(&cfg))
            }
            None => None,
        };

        let smart_wheel = match &self.smart_wheel {
            Some(slots) => {
                if slots.len() != 4 {
                    return Err(format!(
                        "smart_wheel must have exactly 4 entries, got {}",
                        slots.len()
                    ));
                }
                let mut arr = baseline_smart_wheel_slots();
                for (i, s) in slots.iter().enumerate() {
                    arr[i] = SmartWheelSlot {
                        mode: parse_wheel_mode(&s.mode)?,
                        color: parse_color(&s.color)?,
                    };
                }
                Some(build_smart_wheel_page(&arr))
            }
            None => None,
        };

        Ok(CompiledProfile {
            base_keymap,
            fn_keymap,
            rgb,
            smart_wheel,
        })
    }
}

fn compile_keymap_entries(
    section: &str,
    map: &BTreeMap<String, String>,
) -> Result<Vec<(usize, KeyEntry)>, String> {
    let mut entries = Vec::with_capacity(map.len());
    for (key_name, action_str) in map {
        let info = find_key(key_name)
            .ok_or_else(|| format!("{section}: unknown physical key '{key_name}'"))?;
        if !info.remappable {
            return Err(format!(
                "{section}: key '{}' (slot {}) is hardware-locked and cannot be remapped",
                info.name, info.key_index
            ));
        }
        let entry = KeyEntry::parse(action_str)
            .map_err(|e| format!("{section}.{key_name}: {e}"))?;
        entries.push((info.key_index, entry));
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{baseline_base_keymap, baseline_fn_keymap};

    #[test]
    fn baseline_profile_roundtrips_and_matches_captures() {
        let prof = Profile::baseline();
        let toml_str = prof.to_toml_string().unwrap();
        let parsed: Profile = toml::from_str(&toml_str).unwrap();
        let compiled = parsed.compile().unwrap();

        assert_eq!(compiled.base_keymap.unwrap(), baseline_base_keymap());
        assert_eq!(compiled.fn_keymap.unwrap(), baseline_fn_keymap());
        assert_eq!(
            compiled.smart_wheel.unwrap(),
            build_smart_wheel_page(&baseline_smart_wheel_slots())
        );
    }

    #[test]
    fn sample_profiles_parse_and_compile() {
        let def = Profile::load(Path::new("profiles/default.toml")).unwrap();
        let def_c = def.compile().unwrap();
        assert_eq!(def_c.base_keymap.unwrap(), baseline_base_keymap());
        assert_eq!(def_c.fn_keymap.unwrap(), baseline_fn_keymap());
        assert_eq!(
            def_c.smart_wheel.unwrap(),
            build_smart_wheel_page(&baseline_smart_wheel_slots())
        );
        // Active RGB slot matches filtered_apply_default_profile.txt (static blue #0000ff, bright 5, speed 6)
        let rgb = def_c.rgb.unwrap();
        assert_eq!(
            &rgb[17][0..16],
            &[0x01, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x06, 0x00, 0x00, 0x00, 0xaa, 0x55]
        );

        let test = Profile::load(Path::new("profiles/test.toml")).unwrap();
        let test_c = test.compile().unwrap();
        assert!(test_c.base_keymap.is_some());
        assert!(test_c.rgb.is_some());
        assert!(test_c.smart_wheel.is_some());
    }
}
