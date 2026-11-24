use crate::layout::find_key;
use crate::protocol::{
    KEYMAP_PAGES, KeyEntry, RGB_PAGES, RgbConfig, RgbMode, SmartWheelSlot,
    baseline_smart_wheel_slots, build_base_keymap, build_fn_keymap, build_rgb_pages,
    build_smart_wheel_page, format_hex_color, parse_color, parse_wheel_mode, wheel_mode_name,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub rgb: Option<RgbProfile>,
    #[serde(default)]
    pub smart_wheel: Option<Vec<WheelSlotProfile>>,
    #[serde(default)]
    pub keymap: Option<KeymapProfile>,
    /// Host-side actions triggered by `macro:<slot>` keys via Interface 1 Report ID 0x05.
    #[serde(default)]
    pub actions: BTreeMap<String, ActionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionSpec {
    Launch {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    Shell { command: String },
    Text {
        text: String,
        #[serde(default = "default_type_delay_ms")]
        delay_ms: u32,
    },
    Macro {
        steps: Vec<MacroStep>,
        #[serde(default = "default_type_delay_ms")]
        delay_ms: u32,
    },
}

fn default_type_delay_ms() -> u32 {
    12
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MacroStep {
    Key { key: String },
    Text { text: String },
    Delay { delay_ms: u64 },
    Exec {
        exec: String,
        #[serde(default)]
        args: Vec<String>,
    },
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

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create directory '{}': {e}", parent.display()))?;
        }
        let body = self.to_toml_string()?;
        fs::write(path, body)
            .map_err(|e| format!("failed to write profile '{}': {e}", path.display()))
    }
}

/// Default XDG profile directory: `~/.config/c989ctl/profiles`.
pub fn default_profiles_dir() -> PathBuf {
    if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("c989ctl").join("profiles");
        }
    }
    if let Ok(home) = env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home)
                .join(".config")
                .join("c989ctl")
                .join("profiles");
        }
    }
    PathBuf::from("profiles")
}

/// Discover `.toml` profiles in `dir`, returning `(stem, path)` sorted by name.
pub fn discover_profiles(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("profile")
                .to_string();
            out.push((name, path));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

impl Profile {

    /// Build the factory default profile (all base keys at firmware default, factory Fn media keys).
    pub fn baseline() -> Self {
        let base = BTreeMap::new();

        let mut fn_layer = BTreeMap::new();
        fn_layer.insert("f1".into(), "media_player".into());
        fn_layer.insert("f2".into(), "play_pause".into());
        fn_layer.insert("f3".into(), "stop".into());
        fn_layer.insert("f4".into(), "prev_track".into());
        fn_layer.insert("f5".into(), "next_track".into());
        fn_layer.insert("prtsc".into(), "scrlk".into());
        fn_layer.insert("pause".into(), "insert".into());

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
                per_key: BTreeMap::new(),
            }),
            smart_wheel: Some(wheel),
            keymap: Some(KeymapProfile {
                base: Some(base),
                fn_layer: Some(fn_layer),
            }),
            actions: BTreeMap::new(),
        }
    }

    /// Parse `[actions.<slot>]` keys into numeric slot IDs (`1..=255`).
    pub fn parsed_actions(&self) -> Result<BTreeMap<u8, ActionSpec>, String> {
        let mut out = BTreeMap::new();
        for (slot_key, spec) in &self.actions {
            let slot: u8 = if let Some(hex) = slot_key.strip_prefix("0x") {
                u8::from_str_radix(hex, 16)
            } else {
                slot_key.parse::<u8>()
            }
            .map_err(|_| format!("actions.{slot_key}: expected slot number 1..=255"))?;
            if slot == 0 {
                return Err("actions.0: slot 0 is reserved; use slots 1..=255".into());
            }
            out.insert(slot, spec.clone());
        }
        Ok(out)
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
    fn default_profile_and_actions_parse_and_compile() {
        let def = Profile::load(Path::new("profiles/default.toml")).unwrap();
        let def_c = def.compile().unwrap();
        assert_eq!(def_c.base_keymap.unwrap(), baseline_base_keymap());
        assert_eq!(def_c.fn_keymap.unwrap(), baseline_fn_keymap());
        assert_eq!(
            def_c.smart_wheel.unwrap(),
            build_smart_wheel_page(&baseline_smart_wheel_slots())
        );
        let rgb = def_c.rgb.unwrap();
        assert_eq!(
            &rgb[17][0..16],
            &[0x01, 0x00, 0x00, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x06, 0x00, 0x00, 0x00, 0xaa, 0x55]
        );

        let sample = r#"
name = "actions_sample"
[keymap.base]
m6 = "macro:1"
m7 = "macro:2"
[actions.1]
kind = "launch"
command = "krita"
[actions.2]
kind = "macro"
steps = [{ text = "hi" }, { delay_ms = 50 }, { key = "Return" }]
"#;
        let parsed: Profile = toml::from_str(sample).unwrap();
        assert!(parsed.compile().unwrap().base_keymap.is_some());
        let actions = parsed.parsed_actions().unwrap();
        assert_eq!(actions.len(), 2);
        assert!(actions.contains_key(&1));
        assert!(actions.contains_key(&2));
    }
}
