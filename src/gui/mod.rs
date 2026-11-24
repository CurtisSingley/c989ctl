mod keyboard;

use crate::daemon::{DaemonController, DaemonSnapshot, execute_action};
use crate::layout::{
    KEYS, ZONE_ARROWS, ZONE_MACROS, ZONE_NUMBERS, ZONE_NUMPAD, ZONE_WASD, find_key,
};
use crate::profile::{
    ActionSpec, KeymapProfile, MacroStep, Profile, RgbProfile, WheelSlotProfile,
    default_profiles_dir, discover_profiles,
};
use crate::protocol::{
    ALL_RGB_MODES, KEYMAP_PAGES, KeyEntry, RgbMode, baseline_base_keymap, baseline_fn_keymap,
    baseline_smart_wheel_slots, format_hex_color, parse_color, parse_wheel_mode, wheel_mode_name,
};
use crate::transport::{C989, list_interfaces};
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row,
    scrollable, slider, text, text_input, vertical_rule,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Subscription, Task, Theme};
use keyboard::{KeyboardCanvas, KeyboardViewMode};
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

pub fn run() -> Result<(), String> {
    iced::application(App::title, App::update, App::view)
        .subscription(App::subscription)
        .theme(App::theme)
        .window_size((1340.0, 860.0))
        .run_with(App::new)
        .map_err(|e| format!("GUI failed: {e}"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Layout,
    Lighting,
    SmartWheel,
    Macros,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditLayer {
    Base,
    Fn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RgbModeChoice {
    Mode(RgbMode),
}

impl fmt::Display for RgbModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RgbModeChoice::Mode(m) = self;
        write!(f, "{} (0x{:02x})", m.name(), m.id())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WheelModeChoice {
    Volume,
    TaskSwitcher,
    VerticalScroll,
    Brightness,
    HorizontalScroll,
    Zoom,
}

const ALL_WHEEL_MODES: &[WheelModeChoice] = &[
    WheelModeChoice::Volume,
    WheelModeChoice::TaskSwitcher,
    WheelModeChoice::VerticalScroll,
    WheelModeChoice::Brightness,
    WheelModeChoice::HorizontalScroll,
    WheelModeChoice::Zoom,
];

impl WheelModeChoice {
    fn from_id(id: u8) -> Self {
        match id {
            0 => Self::Volume,
            1 => Self::TaskSwitcher,
            2 => Self::VerticalScroll,
            3 => Self::Brightness,
            4 => Self::HorizontalScroll,
            5 => Self::Zoom,
            _ => Self::Volume,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Volume => "volume",
            Self::TaskSwitcher => "task_switcher",
            Self::VerticalScroll => "vertical_scroll",
            Self::Brightness => "brightness",
            Self::HorizontalScroll => "horizontal_scroll",
            Self::Zoom => "zoom",
        }
    }
}

impl fmt::Display for WheelModeChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Volume => "Volume (0x00)",
            Self::TaskSwitcher => "Task Switcher (0x01)",
            Self::VerticalScroll => "Vertical Scroll (0x02)",
            Self::Brightness => "Brightness (0x03)",
            Self::HorizontalScroll => "Horizontal Scroll (0x04)",
            Self::Zoom => "Zoom (0x05)",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionKindChoice {
    Launch,
    Shell,
    Text,
    Macro,
}

const ALL_ACTION_KINDS: &[ActionKindChoice] = &[
    ActionKindChoice::Launch,
    ActionKindChoice::Shell,
    ActionKindChoice::Text,
    ActionKindChoice::Macro,
];

impl fmt::Display for ActionKindChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Launch => write!(f, "Launch App"),
            Self::Shell => write!(f, "Shell Command"),
            Self::Text => write!(f, "Type Text (xdotool)"),
            Self::Macro => write!(f, "Multi-Step Macro"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum LedZone {
    All,
    Wasd,
    Macros,
    Arrows,
    Numpad,
    Numbers,
    Clear,
}

struct PendingRevert {
    seconds_left: u64,
    description: String,
    revert_base: Option<[[u8; 64]; KEYMAP_PAGES]>,
    revert_fn: Option<[[u8; 64]; KEYMAP_PAGES]>,
}

struct App {
    tab: Tab,
    layer: EditLayer,

    // Hardware status
    device_connected: bool,
    status_message: String,
    status_is_error: bool,

    // Profile storage
    profiles_dir: PathBuf,
    profile_entries: Vec<(String, PathBuf)>,
    current_path: Option<PathBuf>,
    profile: Profile,
    dirty: bool,
    new_profile_name: String,

    // Layout editor state
    selected_key: &'static str,
    action_input: String,
    mod_ctrl: bool,
    mod_shift: bool,
    mod_alt: bool,
    mod_win: bool,

    // RGB editor state
    rgb_r: u8,
    rgb_g: u8,
    rgb_b: u8,
    rgb_hex_input: String,

    // Smart Wheel editor state
    selected_wheel_slot: usize,
    wheel_r: u8,
    wheel_g: u8,
    wheel_b: u8,
    wheel_hex_input: String,

    // Action/Macro editor state
    edit_action_slot: String,
    edit_action_kind: ActionKindChoice,
    edit_action_command: String,
    edit_action_args: String,
    edit_action_text: String,
    edit_action_delay: String,
    edit_macro_steps: String,

    // 15-second auto-revert safety net
    pending_revert: Option<PendingRevert>,

    // Opt-in background macro/launcher listener (OFF by default)
    daemon: Option<DaemonController>,
    daemon_snapshot: DaemonSnapshot,
}

#[derive(Debug, Clone)]
enum Message {
    Tick,
    RefreshHardware,
    SelectTab(Tab),
    SelectLayer(EditLayer),

    // Profiles
    LoadProfile(PathBuf),
    SaveProfile,
    NewProfileNameChanged(String),
    CreateProfile,
    DuplicateProfile,
    DeleteCurrentProfile,
    ResetProfileToBaseline,

    // Keyboard canvas interaction
    KeyLeftClicked(&'static str),
    KeyRightClicked(&'static str),

    // Layout editor
    ActionInputChanged(String),
    ToggleModCtrl(bool),
    ToggleModShift(bool),
    ToggleModAlt(bool),
    ToggleModWin(bool),
    ApplyActionInput,
    QuickSetAction(String),
    ClearKeyBinding,

    // RGB editor
    RgbModeSelected(RgbModeChoice),
    RgbRChanged(u8),
    RgbGChanged(u8),
    RgbBChanged(u8),
    RgbHexChanged(String),
    RgbSwatch((u8, u8, u8)),
    RgbBrightnessChanged(u8),
    RgbSpeedChanged(u8),
    RgbFullSpectrumToggled(bool),
    RgbFillZone(LedZone),

    // Smart Wheel editor
    WheelSlotSelected(usize),
    WheelModeSelected(usize, WheelModeChoice),
    WheelRChanged(u8),
    WheelGChanged(u8),
    WheelBChanged(u8),
    WheelHexChanged(String),
    WheelSwatch((u8, u8, u8)),
    ResetWheelDefaults,

    // Macros & Actions editor + opt-in listener
    ToggleDaemon,
    ActionSlotChanged(String),
    ActionKindSelected(ActionKindChoice),
    ActionCommandChanged(String),
    ActionArgsChanged(String),
    ActionTextChanged(String),
    ActionDelayChanged(String),
    ActionStepsChanged(String),
    SaveActionSlot,
    LoadActionIntoEditor(u8),
    DeleteActionSlot(u8),
    TestActionSlot(u8),

    // Hardware writes & safety net
    ApplyFullProfile,
    ApplyLayoutOnly,
    ApplyRgbOnly,
    ApplyWheelOnly,
    ConfirmKeepChanges,
    RevertNow,
    RestoreSafeBaseline,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let profiles_dir = default_profiles_dir();
        let _ = std::fs::create_dir_all(&profiles_dir);

        // Seed default.toml in ~/.config/c989ctl/profiles if empty
        if discover_profiles(&profiles_dir).is_empty() {
            let repo_profiles = PathBuf::from("profiles");
            let repo_list = discover_profiles(&repo_profiles);
            if !repo_list.is_empty() {
                for (stem, src) in repo_list {
                    let dst = profiles_dir.join(format!("{stem}.toml"));
                    let _ = std::fs::copy(src, dst);
                }
            } else {
                let _ = Profile::baseline().save(&profiles_dir.join("default.toml"));
            }
        }

        let profile_entries = discover_profiles(&profiles_dir);
        let default_entry = profile_entries
            .iter()
            .find(|(n, _)| n == "default")
            .or_else(|| profile_entries.first())
            .cloned();

        let (current_path, mut profile) = match default_entry {
            Some((_, path)) => match Profile::load(&path) {
                Ok(p) => (Some(path), p),
                Err(_) => (None, Profile::baseline()),
            },
            None => (None, Profile::baseline()),
        };
        ensure_profile_sections(&mut profile);

        let device_connected = list_interfaces().map(|v| !v.is_empty()).unwrap_or(false);

        let mut app = Self {
            tab: Tab::Layout,
            layer: EditLayer::Base,
            device_connected,
            status_message: if device_connected {
                "Cloud Nine C989 detected (0c45:8006). Ready.".into()
            } else {
                "C989 not detected — check USB connection & udev rule (70-c989.rules).".into()
            },
            status_is_error: !device_connected,
            profiles_dir,
            profile_entries,
            current_path,
            profile,
            dirty: false,
            new_profile_name: String::new(),
            selected_key: "m6",
            action_input: String::new(),
            mod_ctrl: false,
            mod_shift: false,
            mod_alt: false,
            mod_win: false,
            rgb_r: 0,
            rgb_g: 0,
            rgb_b: 255,
            rgb_hex_input: "#0000ff".into(),
            selected_wheel_slot: 0,
            wheel_r: 0xff,
            wheel_g: 0x66,
            wheel_b: 0x00,
            wheel_hex_input: "#ff6600".into(),
            edit_action_slot: "1".into(),
            edit_action_kind: ActionKindChoice::Text,
            edit_action_command: "krita".into(),
            edit_action_args: String::new(),
            edit_action_text: "abcd".into(),
            edit_action_delay: "12".into(),
            edit_macro_steps: "text:hi\ndelay:50\nkey:Return\ntext:c989".into(),
            pending_revert: None,
            daemon: None,
            daemon_snapshot: DaemonSnapshot {
                running: false,
                connected_path: None,
                status_line: "Disabled (opt-in)".into(),
                logs: Vec::new(),
            },
        };
        app.sync_editors_from_profile();
        (app, Task::none())
    }

    fn title(&self) -> String {
        let star = if self.dirty { " *" } else { "" };
        format!("c989ctl — Cloud Nine C989 ErgoFSK [{}]{star}", self.profile.name)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Message> {
        if self.pending_revert.is_some() || self.daemon.is_some() {
            iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn sync_editors_from_profile(&mut self) {
        ensure_profile_sections(&mut self.profile);

        // Sync RGB inputs
        if let Some(rgb) = &self.profile.rgb {
            let (r, g, b) = parse_color(&rgb.color).unwrap_or((0, 0, 255));
            self.rgb_r = r;
            self.rgb_g = g;
            self.rgb_b = b;
            self.rgb_hex_input = format_hex_color((r, g, b));
        }

        // Sync selected wheel slot color
        self.sync_wheel_color_inputs();

        // Sync selected key action input
        self.sync_selected_key_input();
    }

    fn sync_wheel_color_inputs(&mut self) {
        if let Some(slots) = &self.profile.smart_wheel {
            if let Some(slot) = slots.get(self.selected_wheel_slot) {
                let (r, g, b) = parse_color(&slot.color).unwrap_or((0xff, 0x66, 0x00));
                self.wheel_r = r;
                self.wheel_g = g;
                self.wheel_b = b;
                self.wheel_hex_input = format_hex_color((r, g, b));
            }
        }
    }

    fn sync_selected_key_input(&mut self) {
        let current = self
            .active_layer_map()
            .get(self.selected_key)
            .cloned()
            .unwrap_or_default();
        self.action_input = current;
        self.mod_ctrl = false;
        self.mod_shift = false;
        self.mod_alt = false;
        self.mod_win = false;
    }

    fn active_layer_map(&self) -> &BTreeMap<String, String> {
        let km = self.profile.keymap.as_ref().unwrap();
        match self.layer {
            EditLayer::Base => km.base.as_ref().unwrap(),
            EditLayer::Fn => km.fn_layer.as_ref().unwrap(),
        }
    }

    fn active_layer_map_mut(&mut self) -> &mut BTreeMap<String, String> {
        ensure_profile_sections(&mut self.profile);
        let km = self.profile.keymap.as_mut().unwrap();
        match self.layer {
            EditLayer::Base => km.base.as_mut().unwrap(),
            EditLayer::Fn => km.fn_layer.as_mut().unwrap(),
        }
    }

    fn set_ok(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = false;
    }

    fn set_err(&mut self, msg: impl Into<String>) {
        self.status_message = msg.into();
        self.status_is_error = true;
    }

    fn restart_daemon_if_running(&mut self) {
        if self.daemon.is_some() {
            if let Some(old) = self.daemon.take() {
                old.stop();
            }
            match self.profile.parsed_actions() {
                Ok(actions) => {
                    let ctrl = DaemonController::start(self.profile.name.clone(), actions);
                    self.daemon_snapshot = ctrl.snapshot();
                    self.daemon = Some(ctrl);
                }
                Err(e) => {
                    self.set_err(format!("Listener stopped due to action error: {e}"));
                }
            }
        }
    }

    fn perform_revert(&mut self) {
        if let Some(pending) = self.pending_revert.take() {
            match C989::open() {
                Ok(kb) => {
                    let mut err = None;
                    if let Some(base) = pending.revert_base {
                        if let Err(e) = kb.write_base_keymap(&base) {
                            err = Some(e);
                        }
                    }
                    if let Some(fn_map) = pending.revert_fn {
                        if let Err(e) = kb.write_fn_keymap(&fn_map) {
                            err = Some(e);
                        }
                    }
                    match err {
                        Some(e) => self.set_err(format!("Revert failed: {e}")),
                        None => self.set_ok("Reverted keymap cleanly to verified safe baseline."),
                    }
                }
                Err(e) => self.set_err(format!("Revert failed (could not open C989): {e}")),
            }
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if let Some(ctrl) = &self.daemon {
                    self.daemon_snapshot = ctrl.snapshot();
                }
                if let Some(pending) = &mut self.pending_revert {
                    if pending.seconds_left <= 1 {
                        self.perform_revert();
                    } else {
                        pending.seconds_left -= 1;
                    }
                }
            }

            Message::RefreshHardware => {
                self.device_connected = list_interfaces().map(|v| !v.is_empty()).unwrap_or(false);
                if self.device_connected {
                    self.set_ok("Cloud Nine C989 connected (0c45:8006).");
                } else {
                    self.set_err("C989 not found — check USB connection/udev permissions.");
                }
            }

            Message::SelectTab(t) => {
                self.tab = t;
            }

            Message::SelectLayer(l) => {
                self.layer = l;
                self.sync_selected_key_input();
            }

            Message::LoadProfile(path) => match Profile::load(&path) {
                Ok(mut prof) => {
                    ensure_profile_sections(&mut prof);
                    self.profile = prof;
                    self.current_path = Some(path.clone());
                    self.dirty = false;
                    self.sync_editors_from_profile();
                    self.restart_daemon_if_running();
                    self.set_ok(format!("Loaded profile '{}'.", self.profile.name));
                }
                Err(e) => self.set_err(e),
            },

            Message::SaveProfile => {
                let path = self.current_path.clone().unwrap_or_else(|| {
                    self.profiles_dir
                        .join(format!("{}.toml", sanitize_filename(&self.profile.name)))
                });
                match self.profile.save(&path) {
                    Ok(()) => {
                        self.current_path = Some(path);
                        self.dirty = false;
                        self.profile_entries = discover_profiles(&self.profiles_dir);
                        self.set_ok(format!("Saved profile '{}'.", self.profile.name));
                    }
                    Err(e) => self.set_err(e),
                }
            }

            Message::NewProfileNameChanged(s) => {
                self.new_profile_name = s;
            }

            Message::CreateProfile => {
                let raw = self.new_profile_name.trim().to_string();
                if raw.is_empty() {
                    self.set_err("Enter a name for the new profile first.");
                    return Task::none();
                }
                let stem = sanitize_filename(&raw);
                let path = self.profiles_dir.join(format!("{stem}.toml"));
                let mut prof = Profile::baseline();
                prof.name = raw.clone();
                match prof.save(&path) {
                    Ok(()) => {
                        self.profile = prof;
                        self.current_path = Some(path);
                        self.new_profile_name.clear();
                        self.dirty = false;
                        self.profile_entries = discover_profiles(&self.profiles_dir);
                        self.sync_editors_from_profile();
                        self.set_ok(format!("Created profile '{raw}'."));
                    }
                    Err(e) => self.set_err(e),
                }
            }

            Message::DuplicateProfile => {
                let base_name = format!("{}_copy", self.profile.name);
                let stem = sanitize_filename(&base_name);
                let path = self.profiles_dir.join(format!("{stem}.toml"));
                let mut copy = self.profile.clone();
                copy.name = base_name.clone();
                match copy.save(&path) {
                    Ok(()) => {
                        self.profile = copy;
                        self.current_path = Some(path);
                        self.dirty = false;
                        self.profile_entries = discover_profiles(&self.profiles_dir);
                        self.set_ok(format!("Duplicated profile as '{base_name}'."));
                    }
                    Err(e) => self.set_err(e),
                }
            }

            Message::DeleteCurrentProfile => {
                if self.profile_entries.len() <= 1 {
                    self.set_err("Cannot delete the last remaining profile.");
                    return Task::none();
                }
                if let Some(path) = self.current_path.take() {
                    let deleted_name = self.profile.name.clone();
                    match std::fs::remove_file(&path) {
                        Ok(()) => {
                            self.profile_entries = discover_profiles(&self.profiles_dir);
                            if let Some((_, first_path)) = self.profile_entries.first().cloned() {
                                if let Ok(mut prof) = Profile::load(&first_path) {
                                    ensure_profile_sections(&mut prof);
                                    self.profile = prof;
                                    self.current_path = Some(first_path);
                                    self.sync_editors_from_profile();
                                }
                            }
                            self.dirty = false;
                            self.set_ok(format!("Deleted profile '{deleted_name}'."));
                        }
                        Err(e) => self.set_err(format!("Failed to delete profile: {e}")),
                    }
                }
            }

            Message::ResetProfileToBaseline => {
                let name = self.profile.name.clone();
                let mut base = Profile::baseline();
                base.name = name;
                self.profile = base;
                self.dirty = true;
                self.sync_editors_from_profile();
                self.set_ok("Reset editor state to verified safe baseline (not yet saved/applied).");
            }

            Message::KeyLeftClicked(key_name) => {
                self.selected_key = key_name;
                if self.tab == Tab::Lighting {
                    if let Some(rgb) = self.profile.rgb.as_mut() {
                        if RgbMode::parse(&rgb.mode).ok() == Some(RgbMode::UserDefine) {
                            let hex = format_hex_color((self.rgb_r, self.rgb_g, self.rgb_b));
                            rgb.per_key.insert(key_name.to_string(), hex.clone());
                            self.dirty = true;
                            self.set_ok(format!("Painted LED '{key_name}' -> {hex}"));
                            return Task::none();
                        }
                    }
                }
                self.sync_selected_key_input();
            }

            Message::KeyRightClicked(key_name) => {
                if self.tab == Tab::Lighting {
                    if let Some(rgb) = self.profile.rgb.as_mut() {
                        if RgbMode::parse(&rgb.mode).ok() == Some(RgbMode::UserDefine) {
                            rgb.per_key.remove(key_name);
                            self.dirty = true;
                            self.set_ok(format!("Cleared LED '{key_name}' (off)."));
                        }
                    }
                }
            }

            Message::ActionInputChanged(s) => {
                self.action_input = s;
            }
            Message::ToggleModCtrl(v) => self.mod_ctrl = v,
            Message::ToggleModShift(v) => self.mod_shift = v,
            Message::ToggleModAlt(v) => self.mod_alt = v,
            Message::ToggleModWin(v) => self.mod_win = v,

            Message::ApplyActionInput => {
                let raw = self.action_input.trim().to_string();
                let composed = self.compose_with_modifiers(&raw);
                self.set_selected_key_action(&composed);
            }

            Message::QuickSetAction(act) => {
                self.action_input = act.clone();
                self.set_selected_key_action(&act);
            }

            Message::ClearKeyBinding => {
                let key = self.selected_key;
                self.active_layer_map_mut().remove(key);
                self.action_input.clear();
                self.dirty = true;
                self.set_ok(format!("Reset '{key}' on {:?} layer to default.", self.layer));
            }

            Message::RgbModeSelected(RgbModeChoice::Mode(mode)) => {
                if let Some(rgb) = self.profile.rgb.as_mut() {
                    rgb.mode = mode.name().to_string();
                    rgb.full_spectrum = Some(mode.default_full_spectrum());
                    self.dirty = true;
                }
            }

            Message::RgbRChanged(v) => {
                self.rgb_r = v;
                self.update_rgb_color_in_profile();
            }
            Message::RgbGChanged(v) => {
                self.rgb_g = v;
                self.update_rgb_color_in_profile();
            }
            Message::RgbBChanged(v) => {
                self.rgb_b = v;
                self.update_rgb_color_in_profile();
            }
            Message::RgbSwatch((r, g, b)) => {
                self.rgb_r = r;
                self.rgb_g = g;
                self.rgb_b = b;
                self.update_rgb_color_in_profile();
            }
            Message::RgbHexChanged(hex) => {
                self.rgb_hex_input = hex.clone();
                if let Ok((r, g, b)) = parse_color(&hex) {
                    self.rgb_r = r;
                    self.rgb_g = g;
                    self.rgb_b = b;
                    if let Some(rgb) = self.profile.rgb.as_mut() {
                        rgb.color = format_hex_color((r, g, b));
                        self.dirty = true;
                    }
                }
            }
            Message::RgbBrightnessChanged(v) => {
                if let Some(rgb) = self.profile.rgb.as_mut() {
                    rgb.brightness = v;
                    self.dirty = true;
                }
            }
            Message::RgbSpeedChanged(v) => {
                if let Some(rgb) = self.profile.rgb.as_mut() {
                    rgb.speed = v;
                    self.dirty = true;
                }
            }
            Message::RgbFullSpectrumToggled(v) => {
                if let Some(rgb) = self.profile.rgb.as_mut() {
                    rgb.full_spectrum = Some(v);
                    self.dirty = true;
                }
            }
            Message::RgbFillZone(zone) => {
                let hex = format_hex_color((self.rgb_r, self.rgb_g, self.rgb_b));
                if let Some(rgb) = self.profile.rgb.as_mut() {
                    rgb.mode = RgbMode::UserDefine.name().to_string();
                    match zone {
                        LedZone::Clear => rgb.per_key.clear(),
                        LedZone::All => {
                            for k in KEYS {
                                rgb.per_key.insert(k.name.to_string(), hex.clone());
                            }
                        }
                        LedZone::Wasd => fill_zone(&mut rgb.per_key, ZONE_WASD, &hex),
                        LedZone::Macros => fill_zone(&mut rgb.per_key, ZONE_MACROS, &hex),
                        LedZone::Arrows => fill_zone(&mut rgb.per_key, ZONE_ARROWS, &hex),
                        LedZone::Numpad => fill_zone(&mut rgb.per_key, ZONE_NUMPAD, &hex),
                        LedZone::Numbers => fill_zone(&mut rgb.per_key, ZONE_NUMBERS, &hex),
                    }
                    self.dirty = true;
                    self.set_ok("Updated per-key LED zone.");
                }
            }

            Message::WheelSlotSelected(idx) => {
                self.selected_wheel_slot = idx.min(3);
                self.sync_wheel_color_inputs();
            }
            Message::WheelModeSelected(idx, mode) => {
                if let Some(slots) = self.profile.smart_wheel.as_mut() {
                    if let Some(s) = slots.get_mut(idx) {
                        s.mode = mode.as_str().to_string();
                        self.dirty = true;
                    }
                }
            }
            Message::WheelRChanged(v) => {
                self.wheel_r = v;
                self.update_wheel_color_in_profile();
            }
            Message::WheelGChanged(v) => {
                self.wheel_g = v;
                self.update_wheel_color_in_profile();
            }
            Message::WheelBChanged(v) => {
                self.wheel_b = v;
                self.update_wheel_color_in_profile();
            }
            Message::WheelSwatch((r, g, b)) => {
                self.wheel_r = r;
                self.wheel_g = g;
                self.wheel_b = b;
                self.update_wheel_color_in_profile();
            }
            Message::WheelHexChanged(hex) => {
                self.wheel_hex_input = hex.clone();
                if let Ok((r, g, b)) = parse_color(&hex) {
                    self.wheel_r = r;
                    self.wheel_g = g;
                    self.wheel_b = b;
                    if let Some(slots) = self.profile.smart_wheel.as_mut() {
                        if let Some(s) = slots.get_mut(self.selected_wheel_slot) {
                            s.color = format_hex_color((r, g, b));
                            self.dirty = true;
                        }
                    }
                }
            }
            Message::ResetWheelDefaults => {
                self.profile.smart_wheel = Some(
                    baseline_smart_wheel_slots()
                        .into_iter()
                        .map(|s| WheelSlotProfile {
                            mode: wheel_mode_name(s.mode).to_string(),
                            color: format_hex_color(s.color),
                        })
                        .collect(),
                );
                self.sync_wheel_color_inputs();
                self.dirty = true;
                self.set_ok("Reset Smart Wheel slots to defaults (click Apply Wheel Now to flash).");
            }

            Message::ToggleDaemon => {
                if let Some(ctrl) = self.daemon.take() {
                    ctrl.stop();
                    self.daemon_snapshot = DaemonSnapshot {
                        running: false,
                        connected_path: None,
                        status_line: "Disabled (opt-in)".into(),
                        logs: vec!["Listener stopped by user.".into()],
                    };
                    self.set_ok("Macro listener disabled.");
                } else {
                    match self.profile.parsed_actions() {
                        Ok(actions) => {
                            let ctrl = DaemonController::start(self.profile.name.clone(), actions);
                            self.daemon_snapshot = ctrl.snapshot();
                            self.daemon = Some(ctrl);
                            self.set_ok("Macro listener enabled (listening on Interface 1).");
                        }
                        Err(e) => self.set_err(e),
                    }
                }
            }

            Message::ActionSlotChanged(s) => self.edit_action_slot = s,
            Message::ActionKindSelected(k) => self.edit_action_kind = k,
            Message::ActionCommandChanged(s) => self.edit_action_command = s,
            Message::ActionArgsChanged(s) => self.edit_action_args = s,
            Message::ActionTextChanged(s) => self.edit_action_text = s,
            Message::ActionDelayChanged(s) => self.edit_action_delay = s,
            Message::ActionStepsChanged(s) => self.edit_macro_steps = s,

            Message::SaveActionSlot => {
                let slot: u8 = match self.edit_action_slot.trim().parse::<u8>() {
                    Ok(v) if v >= 1 => v,
                    _ => {
                        self.set_err("Slot must be a number between 1 and 255.");
                        return Task::none();
                    }
                };
                let delay_ms = self.edit_action_delay.trim().parse::<u32>().unwrap_or(12);
                let spec = match self.edit_action_kind {
                    ActionKindChoice::Launch => {
                        let cmd = self.edit_action_command.trim().to_string();
                        if cmd.is_empty() {
                            self.set_err("Launch command cannot be empty.");
                            return Task::none();
                        }
                        let args = self
                            .edit_action_args
                            .split_whitespace()
                            .map(|s| s.to_string())
                            .collect();
                        ActionSpec::Launch { command: cmd, args }
                    }
                    ActionKindChoice::Shell => {
                        let cmd = self.edit_action_command.trim().to_string();
                        if cmd.is_empty() {
                            self.set_err("Shell command cannot be empty.");
                            return Task::none();
                        }
                        ActionSpec::Shell { command: cmd }
                    }
                    ActionKindChoice::Text => ActionSpec::Text {
                        text: self.edit_action_text.clone(),
                        delay_ms,
                    },
                    ActionKindChoice::Macro => match parse_macro_steps_text(&self.edit_macro_steps) {
                        Ok(steps) => ActionSpec::Macro { steps, delay_ms },
                        Err(e) => {
                            self.set_err(e);
                            return Task::none();
                        }
                    },
                };
                self.profile.actions.insert(slot.to_string(), spec);
                self.dirty = true;
                self.restart_daemon_if_running();
                self.set_ok(format!(
                    "Saved action slot {slot}. Bind any key to `macro:{slot}` on the Layout tab."
                ));
            }

            Message::LoadActionIntoEditor(slot) => {
                if let Some(spec) = self.profile.actions.get(&slot.to_string()).cloned() {
                    self.edit_action_slot = slot.to_string();
                    match spec {
                        ActionSpec::Launch { command, args } => {
                            self.edit_action_kind = ActionKindChoice::Launch;
                            self.edit_action_command = command;
                            self.edit_action_args = args.join(" ");
                        }
                        ActionSpec::Shell { command } => {
                            self.edit_action_kind = ActionKindChoice::Shell;
                            self.edit_action_command = command;
                        }
                        ActionSpec::Text { text, delay_ms } => {
                            self.edit_action_kind = ActionKindChoice::Text;
                            self.edit_action_text = text;
                            self.edit_action_delay = delay_ms.to_string();
                        }
                        ActionSpec::Macro { steps, delay_ms } => {
                            self.edit_action_kind = ActionKindChoice::Macro;
                            self.edit_action_delay = delay_ms.to_string();
                            self.edit_macro_steps = format_macro_steps_text(&steps);
                        }
                    }
                }
            }

            Message::DeleteActionSlot(slot) => {
                self.profile.actions.remove(&slot.to_string());
                self.dirty = true;
                self.restart_daemon_if_running();
                self.set_ok(format!("Deleted action slot {slot}."));
            }

            Message::TestActionSlot(slot) => {
                if let Some(spec) = self.profile.actions.get(&slot.to_string()).cloned() {
                    match execute_action(slot, &spec) {
                        Ok(()) => self.set_ok(format!("Executed test for action slot {slot}.")),
                        Err(e) => self.set_err(e),
                    }
                }
            }

            Message::ApplyFullProfile => {
                let compiled = match self.profile.compile() {
                    Ok(c) => c,
                    Err(e) => {
                        self.set_err(e);
                        return Task::none();
                    }
                };
                let kb = match C989::open() {
                    Ok(kb) => kb,
                    Err(e) => {
                        self.device_connected = false;
                        self.set_err(e);
                        return Task::none();
                    }
                };
                self.device_connected = true;

                let mut wrote_base = false;
                let mut wrote_fn = false;

                if let Some(base) = &compiled.base_keymap {
                    if let Err(e) = kb.write_base_keymap(base) {
                        self.set_err(format!("Failed writing base keymap: {e}"));
                        return Task::none();
                    }
                    wrote_base = true;
                }
                if let Some(fn_map) = &compiled.fn_keymap {
                    if let Err(e) = kb.write_fn_keymap(fn_map) {
                        self.set_err(format!("Failed writing Fn keymap: {e}"));
                        return Task::none();
                    }
                    wrote_fn = true;
                }
                if let Some(wheel) = &compiled.smart_wheel {
                    if let Err(e) = kb.write_smart_wheel(wheel) {
                        self.set_err(format!("Failed writing Smart Wheel: {e}"));
                        return Task::none();
                    }
                }
                if let Some(rgb) = &compiled.rgb {
                    if let Err(e) = kb.write_rgb(rgb) {
                        self.set_err(format!("Failed writing RGB lighting: {e}"));
                        return Task::none();
                    }
                }

                if wrote_base || wrote_fn {
                    self.pending_revert = Some(PendingRevert {
                        seconds_left: 15,
                        description: format!("profile '{}'", self.profile.name),
                        revert_base: wrote_base.then(baseline_base_keymap),
                        revert_fn: wrote_fn.then(baseline_fn_keymap),
                    });
                    self.set_ok("Applied full profile! Test your keys and click 'Keep Changes' within 15s.");
                } else {
                    self.set_ok(format!("Applied profile '{}'.", self.profile.name));
                }
            }

            Message::ApplyLayoutOnly => {
                let compiled = match self.profile.compile() {
                    Ok(c) => c,
                    Err(e) => {
                        self.set_err(e);
                        return Task::none();
                    }
                };
                let kb = match C989::open() {
                    Ok(kb) => kb,
                    Err(e) => {
                        self.device_connected = false;
                        self.set_err(e);
                        return Task::none();
                    }
                };
                self.device_connected = true;

                match self.layer {
                    EditLayer::Base => {
                        if let Some(base) = &compiled.base_keymap {
                            if let Err(e) = kb.write_base_keymap(base) {
                                self.set_err(e);
                                return Task::none();
                            }
                            self.pending_revert = Some(PendingRevert {
                                seconds_left: 15,
                                description: "Base Layer (04 11)".into(),
                                revert_base: Some(baseline_base_keymap()),
                                revert_fn: None,
                            });
                            self.set_ok("Flashed Base Layer! Click 'Keep Changes' within 15s or it auto-reverts.");
                        }
                    }
                    EditLayer::Fn => {
                        if let Some(fn_map) = &compiled.fn_keymap {
                            if let Err(e) = kb.write_fn_keymap(fn_map) {
                                self.set_err(e);
                                return Task::none();
                            }
                            self.pending_revert = Some(PendingRevert {
                                seconds_left: 15,
                                description: "Fn Layer (04 27)".into(),
                                revert_base: None,
                                revert_fn: Some(baseline_fn_keymap()),
                            });
                            self.set_ok("Flashed Fn Layer! Click 'Keep Changes' within 15s or it auto-reverts.");
                        }
                    }
                }
            }

            Message::ApplyRgbOnly => {
                let compiled = match self.profile.compile() {
                    Ok(c) => c,
                    Err(e) => {
                        self.set_err(e);
                        return Task::none();
                    }
                };
                match C989::open().and_then(|kb| kb.write_rgb(compiled.rgb.as_ref().unwrap())) {
                    Ok(()) => {
                        self.device_connected = true;
                        self.set_ok("Applied RGB lighting (04 13).");
                    }
                    Err(e) => self.set_err(e),
                }
            }

            Message::ApplyWheelOnly => {
                let compiled = match self.profile.compile() {
                    Ok(c) => c,
                    Err(e) => {
                        self.set_err(e);
                        return Task::none();
                    }
                };
                match C989::open()
                    .and_then(|kb| kb.write_smart_wheel(compiled.smart_wheel.as_ref().unwrap()))
                {
                    Ok(()) => {
                        self.device_connected = true;
                        self.set_ok("Applied Smart Wheel settings (04 25).");
                    }
                    Err(e) => self.set_err(e),
                }
            }

            Message::ConfirmKeepChanges => {
                if let Some(p) = self.pending_revert.take() {
                    self.set_ok(format!("Kept changes for {}.", p.description));
                }
            }

            Message::RevertNow => {
                self.perform_revert();
            }

            Message::RestoreSafeBaseline => {
                self.pending_revert = None;
                let res = C989::open().and_then(|kb| {
                    kb.write_base_keymap(&baseline_base_keymap())?;
                    kb.write_fn_keymap(&baseline_fn_keymap())?;
                    let wheel = crate::protocol::build_smart_wheel_page(&baseline_smart_wheel_slots());
                    kb.write_smart_wheel(&wheel)?;
                    Ok(())
                });
                match res {
                    Ok(()) => {
                        self.device_connected = true;
                        self.set_ok("Restored verified safe baseline (Base + Fn + Smart Wheel).");
                    }
                    Err(e) => self.set_err(e),
                }
            }
        }
        Task::none()
    }

    fn compose_with_modifiers(&self, raw: &str) -> String {
        if raw.is_empty()
            || raw.contains('+')
            || raw.starts_with("macro:")
            || raw.starts_with("raw:")
            || raw.starts_with("consumer:")
        {
            return raw.to_string();
        }
        let mut parts = Vec::new();
        if self.mod_ctrl {
            parts.push("ctrl");
        }
        if self.mod_shift {
            parts.push("shift");
        }
        if self.mod_alt {
            parts.push("alt");
        }
        if self.mod_win {
            parts.push("win");
        }
        parts.push(raw);
        parts.join("+")
    }

    fn set_selected_key_action(&mut self, action: &str) {
        let Some(info) = find_key(self.selected_key) else {
            return;
        };
        if !info.remappable {
            self.set_err(format!(
                "Key '{}' (slot {}) is hardware-locked and cannot be remapped.",
                info.name, info.key_index
            ));
            return;
        }
        if action.is_empty() || action.eq_ignore_ascii_case("default") {
            self.active_layer_map_mut().remove(info.name);
            self.dirty = true;
            self.set_ok(format!("Set '{}' to firmware default.", info.name));
            return;
        }
        match KeyEntry::parse(action) {
            Ok(entry) => {
                self.active_layer_map_mut()
                    .insert(info.name.to_string(), action.to_string());
                self.dirty = true;
                self.set_ok(format!(
                    "Mapped '{}' -> '{}' ({:02x?}) [not yet flashed].",
                    info.name,
                    action,
                    entry.to_bytes()
                ));
            }
            Err(e) => self.set_err(format!("Invalid action '{action}': {e}")),
        }
    }

    fn update_rgb_color_in_profile(&mut self) {
        let hex = format_hex_color((self.rgb_r, self.rgb_g, self.rgb_b));
        self.rgb_hex_input = hex.clone();
        if let Some(rgb) = self.profile.rgb.as_mut() {
            rgb.color = hex;
            self.dirty = true;
        }
    }

    fn update_wheel_color_in_profile(&mut self) {
        let hex = format_hex_color((self.wheel_r, self.wheel_g, self.wheel_b));
        self.wheel_hex_input = hex.clone();
        if let Some(slots) = self.profile.smart_wheel.as_mut() {
            if let Some(s) = slots.get_mut(self.selected_wheel_slot) {
                s.color = hex;
                self.dirty = true;
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let sidebar = self.view_sidebar();
        let main_area = self.view_main();

        row![sidebar, vertical_rule(1), main_area]
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        let hw_badge = if self.device_connected {
            text("● C989 Connected").size(13).color(Color::from_rgb8(80, 220, 120))
        } else {
            text("○ C989 Disconnected").size(13).color(Color::from_rgb8(240, 110, 100))
        };

        let nav = column![
            nav_button("Layout (Base / Fn)", Tab::Layout, self.tab),
            nav_button("Lighting (RGB)", Tab::Lighting, self.tab),
            nav_button("Smart Wheel", Tab::SmartWheel, self.tab),
            nav_button("Macros & Actions", Tab::Macros, self.tab),
        ]
        .spacing(6);

        let mut prof_list = column![].spacing(4);
        for (name, path) in &self.profile_entries {
            let is_active = self.current_path.as_ref() == Some(path);
            let label = if is_active && self.dirty {
                format!("{name} *")
            } else {
                name.clone()
            };
            let btn = button(text(label).size(13))
                .width(Length::Fill)
                .style(if is_active {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::LoadProfile(path.clone()));
            prof_list = prof_list.push(btn);
        }

        let new_prof_row = row![
            text_input("New profile...", &self.new_profile_name)
                .on_input(Message::NewProfileNameChanged)
                .on_submit(Message::CreateProfile)
                .size(13)
                .width(Length::Fill),
            button(text("+").size(13))
                .on_press(Message::CreateProfile)
                .style(button::primary),
        ]
        .spacing(4);

        let prof_actions = row![
            button(text("Save").size(12))
                .on_press(Message::SaveProfile)
                .style(button::success),
            button(text("Dup").size(12))
                .on_press(Message::DuplicateProfile)
                .style(button::secondary),
            button(text("Del").size(12))
                .on_press(Message::DeleteCurrentProfile)
                .style(button::danger),
        ]
        .spacing(4);

        let listener_badge = if self.daemon.is_some() {
            text("Listener: ON").size(12).color(Color::from_rgb8(80, 220, 180))
        } else {
            text("Listener: OFF (opt-in)")
                .size(12)
                .color(Color::from_rgb8(150, 156, 170))
        };

        let content = column![
            text("c989ctl").size(22),
            text("Cloud Nine C989 ErgoFSK")
                .size(12)
                .color(Color::from_rgb8(150, 158, 172)),
            row![
                hw_badge,
                horizontal_space(),
                button(text("↻").size(12))
                    .on_press(Message::RefreshHardware)
                    .style(button::secondary),
            ]
            .align_y(Alignment::Center),
            horizontal_rule(1),
            nav,
            horizontal_rule(1),
            text("Profiles").size(14),
            scrollable(prof_list).height(Length::FillPortion(1)),
            new_prof_row,
            prof_actions,
            button(text("Reset Editor to Baseline").size(12))
                .width(Length::Fill)
                .style(button::secondary)
                .on_press(Message::ResetProfileToBaseline),
            horizontal_rule(1),
            listener_badge,
            button(text(if self.daemon.is_some() {
                "Stop Macro Listener"
            } else {
                "Enable Macro Listener"
            })
            .size(12))
            .width(Length::Fill)
            .style(if self.daemon.is_some() {
                button::danger
            } else {
                button::secondary
            })
            .on_press(Message::ToggleDaemon),
            horizontal_rule(1),
            button(text("Apply Full Profile").size(14))
                .width(Length::Fill)
                .style(button::primary)
                .on_press(Message::ApplyFullProfile),
            button(text("Restore Safe Baseline").size(12))
                .width(Length::Fill)
                .style(button::danger)
                .on_press(Message::RestoreSafeBaseline),
        ]
        .spacing(8)
        .padding(12)
        .width(Length::Fixed(235.0))
        .height(Length::Fill);

        container(content).into()
    }

    fn view_main(&self) -> Element<'_, Message> {
        let mut col = column![].spacing(10).padding(14).width(Length::Fill).height(Length::Fill);

        // 15-second auto-revert safety banner (always at top when active!)
        if let Some(pending) = &self.pending_revert {
            let banner = container(
                row![
                    text(format!(
                        "SAFETY NET: Flashed {}. Auto-reverting to safe baseline in {}s...",
                        pending.description, pending.seconds_left
                    ))
                    .size(14)
                    .color(Color::WHITE),
                    horizontal_space(),
                    button(text("Keep Changes").size(13))
                        .style(button::success)
                        .on_press(Message::ConfirmKeepChanges),
                    button(text("Revert Now").size(13))
                        .style(button::danger)
                        .on_press(Message::RevertNow),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .style(|_theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(165, 62, 24))),
                border: Border {
                    color: Color::from_rgb8(255, 165, 60),
                    width: 1.5,
                    radius: 6.0.into(),
                },
                ..Default::default()
            });
            col = col.push(banner);
        }

        // Visual interactive C989 keyboard canvas
        let rgb_prof = self.profile.rgb.as_ref().unwrap();
        let rgb_mode = RgbMode::parse(&rgb_prof.mode).unwrap_or(RgbMode::Static);
        let rgb_color = parse_color(&rgb_prof.color).unwrap_or((0, 0, 255));
        let rgb_full_spec = rgb_prof
            .full_spectrum
            .unwrap_or_else(|| rgb_mode.default_full_spectrum());

        let kb_view_mode = match self.tab {
            Tab::Layout => KeyboardViewMode::Layout,
            Tab::Lighting => KeyboardViewMode::Lighting,
            Tab::SmartWheel | Tab::Macros => KeyboardViewMode::WheelOrMacro,
        };

        let canvas = KeyboardCanvas {
            view_mode: kb_view_mode,
            selected_key: Some(self.selected_key),
            active_bindings: self.active_layer_map(),
            rgb_mode,
            rgb_color,
            rgb_full_spectrum: rgb_full_spec,
            per_key_colors: &rgb_prof.per_key,
            on_left_click: Box::new(Message::KeyLeftClicked),
            on_right_click: Box::new(Message::KeyRightClicked),
        }
        .view();

        col = col.push(canvas);
        col = col.push(horizontal_rule(1));

        // Tab editor body
        let body = match self.tab {
            Tab::Layout => self.view_layout_tab(),
            Tab::Lighting => self.view_lighting_tab(),
            Tab::SmartWheel => self.view_wheel_tab(),
            Tab::Macros => self.view_macros_tab(),
        };
        col = col.push(scrollable(body).height(Length::Fill));

        // Status bar
        let status_color = if self.status_is_error {
            Color::from_rgb8(255, 120, 110)
        } else {
            Color::from_rgb8(130, 220, 170)
        };
        col = col.push(horizontal_rule(1));
        col = col.push(
            row![
                text(&self.status_message).size(13).color(status_color),
                horizontal_space(),
                text(format!("Profiles dir: {}", self.profiles_dir.display()))
                    .size(11)
                    .color(Color::from_rgb8(120, 126, 140)),
            ]
            .align_y(Alignment::Center),
        );

        col.into()
    }

    fn view_layout_tab(&self) -> Element<'_, Message> {
        let key_info = find_key(self.selected_key).unwrap();
        let current_binding = self
            .active_layer_map()
            .get(key_info.name)
            .map(|s| s.as_str())
            .unwrap_or("(firmware default)");

        let layer_bar = row![
            text("Active Layer:").size(14),
            button(text("Base Layer (04 11)").size(13))
                .style(if self.layer == EditLayer::Base {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SelectLayer(EditLayer::Base)),
            button(text("Fn Layer (04 27)").size(13))
                .style(if self.layer == EditLayer::Fn {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Message::SelectLayer(EditLayer::Fn)),
            horizontal_space(),
            button(text("Apply Active Layer Only (15s safety net)").size(13))
                .style(button::primary)
                .on_press(Message::ApplyLayoutOnly),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let composed_preview = self.compose_with_modifiers(self.action_input.trim());
        let validation = if composed_preview.is_empty() {
            text("Empty = firmware default [00, 00, 00, 00]")
                .size(12)
                .color(Color::from_rgb8(150, 158, 172))
        } else {
            match KeyEntry::parse(&composed_preview) {
                Ok(e) => text(format!("Valid: '{}' -> {:02x?}", composed_preview, e.to_bytes()))
                    .size(12)
                    .color(Color::from_rgb8(95, 220, 140)),
                Err(err) => text(format!("Invalid: {err}"))
                    .size(12)
                    .color(Color::from_rgb8(255, 120, 110)),
            }
        };

        let header = row![
            text(format!(
                "Selected Key: {} (\"{}\", slot #{}, default HID 0x{:02x})",
                key_info.name, key_info.label, key_info.key_index, key_info.default_code
            ))
            .size(15),
            horizontal_space(),
            text(format!("Current: {current_binding}"))
                .size(13)
                .color(Color::from_rgb8(255, 185, 95)),
        ];

        if !key_info.remappable {
            return column![
                layer_bar,
                header,
                text("This key (M5/Fn, M10/Light, or Right Fn) is hardware-locked on the C989 and cannot be remapped.")
                    .size(14)
                    .color(Color::from_rgb8(240, 120, 120)),
            ]
            .spacing(12)
            .into();
        }

        let input_row = row![
            text_input(
                "Action (e.g. a, ctrl+c, backspace, mouse_left, play_pause, macro:1, disabled)",
                &self.action_input,
            )
            .on_input(Message::ActionInputChanged)
            .on_submit(Message::ApplyActionInput)
            .width(Length::Fill),
            button(text("Set Key").size(13))
                .style(button::success)
                .on_press(Message::ApplyActionInput),
            button(text("Reset Default").size(13))
                .style(button::secondary)
                .on_press(Message::ClearKeyBinding),
        ]
        .spacing(8);

        let mods_row = row![
            text("Modifiers:").size(13),
            checkbox("Ctrl", self.mod_ctrl).on_toggle(Message::ToggleModCtrl),
            checkbox("Shift", self.mod_shift).on_toggle(Message::ToggleModShift),
            checkbox("Alt", self.mod_alt).on_toggle(Message::ToggleModAlt),
            checkbox("Win/Super", self.mod_win).on_toggle(Message::ToggleModWin),
            horizontal_space(),
            validation,
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let quick_mouse = row![
            text("Mouse:").size(13).width(Length::Fixed(70.0)),
            quick_btn("Left Click", "mouse_left"),
            quick_btn("Right Click", "mouse_right"),
            quick_btn("Middle Click", "mouse_middle"),
            quick_btn("Scroll Up", "scroll_up"),
            quick_btn("Scroll Down", "scroll_down"),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let quick_media = row![
            text("Media:").size(13).width(Length::Fixed(70.0)),
            quick_btn("Play/Pause", "play_pause"),
            quick_btn("Mute", "mute"),
            quick_btn("Vol+", "vol_up"),
            quick_btn("Vol-", "vol_down"),
            quick_btn("Prev", "prev_track"),
            quick_btn("Next", "next_track"),
            quick_btn("Stop", "stop"),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let quick_common = row![
            text("Common:").size(13).width(Length::Fixed(70.0)),
            quick_btn("Caps", "caps"),
            quick_btn("Backspace", "backspace"),
            quick_btn("Esc", "esc"),
            quick_btn("Enter", "enter"),
            quick_btn("Disabled", "disabled"),
            quick_btn("Macro #1", "macro:1"),
            quick_btn("Macro #2", "macro:2"),
            quick_btn("Macro #3", "macro:3"),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        column![
            layer_bar,
            header,
            input_row,
            mods_row,
            quick_mouse,
            quick_media,
            quick_common,
            text("Note: Keyboard outputs standard USB HID codes; your OS XKB layout (e.g. Dvorak, Caps->Esc) translates them.")
                .size(12)
                .color(Color::from_rgb8(140, 148, 162)),
        ]
        .spacing(10)
        .into()
    }

    fn view_lighting_tab(&self) -> Element<'_, Message> {
        let rgb = self.profile.rgb.as_ref().unwrap();
        let mode = RgbMode::parse(&rgb.mode).unwrap_or(RgbMode::Static);
        let full_spec = rgb
            .full_spectrum
            .unwrap_or_else(|| mode.default_full_spectrum());

        let mode_choices: Vec<RgbModeChoice> =
            ALL_RGB_MODES.iter().copied().map(RgbModeChoice::Mode).collect();

        let top_bar = row![
            text("Effect Mode:").size(14),
            pick_list(mode_choices, Some(RgbModeChoice::Mode(mode)), Message::RgbModeSelected)
                .width(Length::Fixed(220.0)),
            checkbox("Full Spectrum (Rainbow)", full_spec).on_toggle(Message::RgbFullSpectrumToggled),
            horizontal_space(),
            button(text("Apply RGB Now").size(13))
                .style(button::primary)
                .on_press(Message::ApplyRgbOnly),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        let preview = color_box((self.rgb_r, self.rgb_g, self.rgb_b), 48.0, 48.0);
        let swatches = row![
            swatch_btn((255, 0, 0), Message::RgbSwatch),
            swatch_btn((0, 255, 0), Message::RgbSwatch),
            swatch_btn((0, 0, 255), Message::RgbSwatch),
            swatch_btn((255, 255, 255), Message::RgbSwatch),
            swatch_btn((255, 255, 0), Message::RgbSwatch),
            swatch_btn((0, 255, 255), Message::RgbSwatch),
            swatch_btn((255, 0, 255), Message::RgbSwatch),
            swatch_btn((255, 102, 0), Message::RgbSwatch),
            swatch_btn((160, 32, 240), Message::RgbSwatch),
            swatch_btn((0, 0, 0), Message::RgbSwatch),
        ]
        .spacing(6);

        let sliders_col = column![
            row![
                text(format!("R: {:3}", self.rgb_r)).size(13).width(Length::Fixed(52.0)),
                slider(0..=255, self.rgb_r, Message::RgbRChanged).width(Length::Fixed(240.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text(format!("G: {:3}", self.rgb_g)).size(13).width(Length::Fixed(52.0)),
                slider(0..=255, self.rgb_g, Message::RgbGChanged).width(Length::Fixed(240.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text(format!("B: {:3}", self.rgb_b)).size(13).width(Length::Fixed(52.0)),
                slider(0..=255, self.rgb_b, Message::RgbBChanged).width(Length::Fixed(240.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text("Hex:").size(13).width(Length::Fixed(52.0)),
                text_input("#RRGGBB", &self.rgb_hex_input)
                    .on_input(Message::RgbHexChanged)
                    .width(Length::Fixed(110.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(6);

        let params_col = column![
            text(format!("Brightness: {}  (0-1 off, 5-6 max)", rgb.brightness)).size(13),
            slider(0..=6, rgb.brightness, Message::RgbBrightnessChanged).width(Length::Fixed(240.0)),
            text(format!("Speed: {}  (1 paused, 10 fastest)", rgb.speed)).size(13),
            slider(1..=10, rgb.speed.clamp(1, 10), Message::RgbSpeedChanged)
                .width(Length::Fixed(240.0)),
        ]
        .spacing(6);

        let controls = row![
            column![row![preview, swatches].spacing(10).align_y(Alignment::Center), sliders_col]
                .spacing(10),
            horizontal_space(),
            params_col,
        ]
        .spacing(20);

        let per_key_bar = row![
            text(if mode == RgbMode::UserDefine {
                "Per-Key Painter (Left-click key to paint, Right-click to clear):"
            } else {
                "Quick Per-Key Zones (switches to User Define 0x80):"
            })
            .size(13),
            button(text("All").size(12)).on_press(Message::RgbFillZone(LedZone::All)),
            button(text("WASD").size(12)).on_press(Message::RgbFillZone(LedZone::Wasd)),
            button(text("Macros").size(12)).on_press(Message::RgbFillZone(LedZone::Macros)),
            button(text("Arrows").size(12)).on_press(Message::RgbFillZone(LedZone::Arrows)),
            button(text("Numpad").size(12)).on_press(Message::RgbFillZone(LedZone::Numpad)),
            button(text("0-9").size(12)).on_press(Message::RgbFillZone(LedZone::Numbers)),
            button(text("Clear").size(12))
                .style(button::danger)
                .on_press(Message::RgbFillZone(LedZone::Clear)),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        column![top_bar, controls, horizontal_rule(1), per_key_bar]
            .spacing(12)
            .into()
    }

    fn view_wheel_tab(&self) -> Element<'_, Message> {
        let slots = self.profile.smart_wheel.as_ref().unwrap();
        let top_bar = row![
            text("Smart Wheel (04 25 — applies immediately without 04 f0)").size(15),
            horizontal_space(),
            button(text("Reset Defaults").size(13))
                .style(button::secondary)
                .on_press(Message::ResetWheelDefaults),
            button(text("Apply Wheel Now").size(13))
                .style(button::primary)
                .on_press(Message::ApplyWheelOnly),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let mut slots_row = row![].spacing(12);
        for (idx, s) in slots.iter().enumerate() {
            let mode_id = parse_wheel_mode(&s.mode).unwrap_or(0);
            let mode_choice = WheelModeChoice::from_id(mode_id);
            let rgb = parse_color(&s.color).unwrap_or((255, 255, 255));
            let is_sel = self.selected_wheel_slot == idx;

            let card = container(
                column![
                    row![
                        text(format!("Slot {}", idx + 1)).size(14),
                        horizontal_space(),
                        color_box(rgb, 24.0, 24.0),
                    ]
                    .align_y(Alignment::Center),
                    pick_list(ALL_WHEEL_MODES, Some(mode_choice), move |m| {
                        Message::WheelModeSelected(idx, m)
                    })
                    .width(Length::Fill),
                    button(text(if is_sel { "Editing Color" } else { "Edit Color" }).size(12))
                        .width(Length::Fill)
                        .style(if is_sel {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .on_press(Message::WheelSlotSelected(idx)),
                ]
                .spacing(8),
            )
            .padding(10)
            .width(Length::FillPortion(1))
            .style(move |_theme| container::Style {
                background: Some(Background::Color(Color::from_rgb8(34, 38, 46))),
                border: Border {
                    color: if is_sel {
                        Color::from_rgb8(56, 224, 245)
                    } else {
                        Color::from_rgb8(62, 68, 82)
                    },
                    width: if is_sel { 2.0 } else { 1.0 },
                    radius: 6.0.into(),
                },
                ..Default::default()
            });
            slots_row = slots_row.push(card);
        }

        let color_editor = row![
            color_box((self.wheel_r, self.wheel_g, self.wheel_b), 52.0, 52.0),
            column![
                text(format!("Slot {} Color", self.selected_wheel_slot + 1)).size(14),
                row![
                    text(format!("R: {:3}", self.wheel_r)).size(13).width(Length::Fixed(52.0)),
                    slider(0..=255, self.wheel_r, Message::WheelRChanged).width(Length::Fixed(220.0)),
                ]
                .spacing(8),
                row![
                    text(format!("G: {:3}", self.wheel_g)).size(13).width(Length::Fixed(52.0)),
                    slider(0..=255, self.wheel_g, Message::WheelGChanged).width(Length::Fixed(220.0)),
                ]
                .spacing(8),
                row![
                    text(format!("B: {:3}", self.wheel_b)).size(13).width(Length::Fixed(52.0)),
                    slider(0..=255, self.wheel_b, Message::WheelBChanged).width(Length::Fixed(220.0)),
                ]
                .spacing(8),
            ]
            .spacing(4),
            column![
                row![
                    text("Hex:").size(13),
                    text_input("#RRGGBB", &self.wheel_hex_input)
                        .on_input(Message::WheelHexChanged)
                        .width(Length::Fixed(100.0)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                row![
                    swatch_btn((255, 102, 0), Message::WheelSwatch),
                    swatch_btn((255, 32, 32), Message::WheelSwatch),
                    swatch_btn((76, 32, 255), Message::WheelSwatch),
                    swatch_btn((32, 255, 32), Message::WheelSwatch),
                    swatch_btn((0, 255, 255), Message::WheelSwatch),
                    swatch_btn((255, 0, 255), Message::WheelSwatch),
                ]
                .spacing(6),
            ]
            .spacing(10),
        ]
        .spacing(16)
        .align_y(Alignment::Center);

        column![top_bar, slots_row, horizontal_rule(1), color_editor]
            .spacing(12)
            .into()
    }

    fn view_macros_tab(&self) -> Element<'_, Message> {
        let daemon_running = self.daemon.is_some();
        let listener_card = container(
            column![
                row![
                    text("Opt-In Host Macro & App-Launcher Listener").size(15),
                    horizontal_space(),
                    text(&self.daemon_snapshot.status_line)
                        .size(13)
                        .color(if daemon_running {
                            Color::from_rgb8(90, 225, 175)
                        } else {
                            Color::from_rgb8(160, 166, 180)
                        }),
                    button(text(if daemon_running { "Disable Listener" } else { "Enable Listener" }).size(13))
                        .style(if daemon_running { button::danger } else { button::success })
                        .on_press(Message::ToggleDaemon),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
                text("Off by default. Onboard remaps, RGB, and Smart Wheel require NO listener. Enable this only when using `macro:<slot>` host actions (Arch/X11/i3 via `xdotool`; Wayland not yet supported).")
                    .size(12)
                    .color(Color::from_rgb8(150, 158, 172)),
            ]
            .spacing(6),
        )
        .padding(10)
        .width(Length::Fill)
        .style(|_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(32, 36, 44))),
            border: Border {
                color: Color::from_rgb8(60, 68, 82),
                width: 1.0,
                radius: 6.0.into(),
            },
            ..Default::default()
        });

        // Configured action slots list
        let mut actions_list = column![text("Configured Action Slots (`[actions.<slot>]`):").size(14)].spacing(6);
        let parsed = self.profile.parsed_actions().unwrap_or_default();
        if parsed.is_empty() {
            actions_list = actions_list.push(
                text("No host actions configured in this profile yet.")
                    .size(13)
                    .color(Color::from_rgb8(140, 146, 160)),
            );
        } else {
            for (slot, spec) in &parsed {
                let summary = match spec {
                    ActionSpec::Launch { command, args } => {
                        format!("Launch: {command} {}", args.join(" "))
                    }
                    ActionSpec::Shell { command } => format!("Shell: {command}"),
                    ActionSpec::Text { text, .. } => format!("Text: {:?}", text),
                    ActionSpec::Macro { steps, .. } => format!("Macro: {} step(s)", steps.len()),
                };
                actions_list = actions_list.push(
                    row![
                        text(format!("Slot {slot:3}")).size(13).width(Length::Fixed(64.0)),
                        text(summary).size(13).width(Length::Fill),
                        button(text("Test").size(12))
                            .style(button::secondary)
                            .on_press(Message::TestActionSlot(*slot)),
                        button(text("Edit").size(12))
                            .style(button::primary)
                            .on_press(Message::LoadActionIntoEditor(*slot)),
                        button(text("Del").size(12))
                            .style(button::danger)
                            .on_press(Message::DeleteActionSlot(*slot)),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                );
            }
        }

        // Action editor form
        let mut editor = column![
            text("Add / Edit Action Slot").size(14),
            row![
                text("Slot (1-255):").size(13),
                text_input("1", &self.edit_action_slot)
                    .on_input(Message::ActionSlotChanged)
                    .width(Length::Fixed(70.0)),
                text("Kind:").size(13),
                pick_list(
                    ALL_ACTION_KINDS,
                    Some(self.edit_action_kind),
                    Message::ActionKindSelected,
                )
                .width(Length::Fixed(200.0)),
                horizontal_space(),
                button(text("Save Action Slot").size(13))
                    .style(button::success)
                    .on_press(Message::SaveActionSlot),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8);

        match self.edit_action_kind {
            ActionKindChoice::Launch => {
                editor = editor.push(
                    row![
                        text("Command:").size(13).width(Length::Fixed(80.0)),
                        text_input("krita", &self.edit_action_command)
                            .on_input(Message::ActionCommandChanged)
                            .width(Length::FillPortion(1)),
                        text("Args:").size(13),
                        text_input("(optional space-separated)", &self.edit_action_args)
                            .on_input(Message::ActionArgsChanged)
                            .width(Length::FillPortion(1)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            ActionKindChoice::Shell => {
                editor = editor.push(
                    row![
                        text("sh -c:").size(13).width(Length::Fixed(80.0)),
                        text_input("i3-msg workspace 1", &self.edit_action_command)
                            .on_input(Message::ActionCommandChanged)
                            .width(Length::Fill),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            ActionKindChoice::Text => {
                editor = editor.push(
                    row![
                        text("Text:").size(13).width(Length::Fixed(80.0)),
                        text_input("Text to type...", &self.edit_action_text)
                            .on_input(Message::ActionTextChanged)
                            .width(Length::Fill),
                        text("Delay (ms):").size(13),
                        text_input("12", &self.edit_action_delay)
                            .on_input(Message::ActionDelayChanged)
                            .width(Length::Fixed(60.0)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
            ActionKindChoice::Macro => {
                editor = editor.push(
                    row![
                        text("Steps (;-separated, e.g. text:hi; delay:50; key:Return; text:c989):")
                            .size(12)
                            .color(Color::from_rgb8(165, 172, 185)),
                    ],
                );
                editor = editor.push(
                    row![
                        text_input(
                            "text:hi; delay:50; key:Return; text:c989",
                            &self.edit_macro_steps,
                        )
                        .on_input(Message::ActionStepsChanged)
                        .width(Length::Fill),
                        text("Delay (ms):").size(13),
                        text_input("12", &self.edit_action_delay)
                            .on_input(Message::ActionDelayChanged)
                            .width(Length::Fixed(60.0)),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
        }

        let mut col = column![listener_card, actions_list, horizontal_rule(1), editor].spacing(10);

        if !self.daemon_snapshot.logs.is_empty() {
            let mut log_col = column![text("Recent Listener Activity:").size(13)].spacing(2);
            for line in self.daemon_snapshot.logs.iter().rev().take(6) {
                log_col = log_col.push(
                    text(line)
                        .size(12)
                        .color(Color::from_rgb8(150, 195, 210)),
                );
            }
            col = col.push(horizontal_rule(1)).push(log_col);
        }

        col.into()
    }
}

fn nav_button(label: &str, tab: Tab, current: Tab) -> Element<'_, Message> {
    button(text(label).size(13))
        .width(Length::Fill)
        .style(if tab == current {
            button::primary
        } else {
            button::secondary
        })
        .on_press(Message::SelectTab(tab))
        .into()
}

fn quick_btn<'a>(label: &'a str, action: &'a str) -> Element<'a, Message> {
    button(text(label).size(12))
        .style(button::secondary)
        .on_press(Message::QuickSetAction(action.to_string()))
        .into()
}

fn color_box<'a>(rgb: (u8, u8, u8), w: f32, h: f32) -> Element<'a, Message> {
    container(text(""))
        .width(Length::Fixed(w))
        .height(Length::Fixed(h))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(Color::from_rgb8(rgb.0, rgb.1, rgb.2))),
            border: Border {
                color: Color::from_rgb8(180, 188, 204),
                width: 1.0,
                radius: 5.0.into(),
            },
            ..Default::default()
        })
        .into()
}

fn swatch_btn<'a>(
    rgb: (u8, u8, u8),
    msg: fn((u8, u8, u8)) -> Message,
) -> Element<'a, Message> {
    button(color_box(rgb, 22.0, 22.0))
        .padding(1)
        .style(button::text)
        .on_press(msg(rgb))
        .into()
}

fn fill_zone(per_key: &mut BTreeMap<String, String>, keys: &[&str], hex: &str) {
    for &k in keys {
        per_key.insert(k.to_string(), hex.to_string());
    }
}

fn ensure_profile_sections(profile: &mut Profile) {
    if profile.rgb.is_none() {
        profile.rgb = Some(RgbProfile {
            mode: "static".into(),
            color: "#0000ff".into(),
            brightness: 5,
            speed: 6,
            full_spectrum: Some(false),
            per_key: BTreeMap::new(),
        });
    }
    if profile.smart_wheel.as_ref().map(|v| v.len()).unwrap_or(0) != 4 {
        profile.smart_wheel = Some(
            baseline_smart_wheel_slots()
                .into_iter()
                .map(|s| WheelSlotProfile {
                    mode: wheel_mode_name(s.mode).to_string(),
                    color: format_hex_color(s.color),
                })
                .collect(),
        );
    }
    let km = profile.keymap.get_or_insert_with(KeymapProfile::default);
    if km.base.is_none() {
        km.base = Some(BTreeMap::new());
    }
    if km.fn_layer.is_none() {
        km.fn_layer = Some(BTreeMap::new());
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if cleaned.is_empty() {
        "profile".into()
    } else {
        cleaned
    }
}

fn parse_macro_steps_text(input: &str) -> Result<Vec<MacroStep>, String> {
    let mut steps = Vec::new();
    for raw in input.split(['\n', ';']) {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        let Some((kind, val)) = item.split_once(':') else {
            return Err(format!(
                "invalid macro step '{item}' (expected text:..., key:..., delay:..., or exec:...)"
            ));
        };
        let val = val.trim();
        match kind.trim().to_lowercase().as_str() {
            "text" => steps.push(MacroStep::Text {
                text: val.to_string(),
            }),
            "key" => steps.push(MacroStep::Key {
                key: val.to_string(),
            }),
            "delay" => {
                let ms = val
                    .parse::<u64>()
                    .map_err(|_| format!("invalid delay ms '{val}'"))?;
                steps.push(MacroStep::Delay { delay_ms: ms });
            }
            "exec" => {
                let mut parts = val.split_whitespace();
                let prog = parts
                    .next()
                    .ok_or_else(|| "exec step requires a command".to_string())?;
                steps.push(MacroStep::Exec {
                    exec: prog.to_string(),
                    args: parts.map(|s| s.to_string()).collect(),
                });
            }
            other => return Err(format!("unknown macro step type '{other}'")),
        }
    }
    if steps.is_empty() {
        return Err("macro must contain at least one step".into());
    }
    Ok(steps)
}

fn format_macro_steps_text(steps: &[MacroStep]) -> String {
    steps
        .iter()
        .map(|s| match s {
            MacroStep::Key { key } => format!("key:{key}"),
            MacroStep::Text { text } => format!("text:{text}"),
            MacroStep::Delay { delay_ms } => format!("delay:{delay_ms}"),
            MacroStep::Exec { exec, args } => {
                if args.is_empty() {
                    format!("exec:{exec}")
                } else {
                    format!("exec:{exec} {}", args.join(" "))
                }
            }
        })
        .collect::<Vec<_>>()
        .join("; ")
}
