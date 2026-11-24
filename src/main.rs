use c989ctl::{daemon, layout, profile, protocol, transport};

use clap::{Parser, Subcommand, ValueEnum};
use layout::{KEYS, find_key};
use profile::Profile;
use protocol::{
    ALL_RGB_MODES, KeyEntry, RgbConfig, RgbMode, baseline_base_keymap, baseline_fn_keymap,
    baseline_smart_wheel_slots, build_rgb_pages, build_smart_wheel_page, format_hex_color,
    parse_color, parse_wheel_mode, set_keymap_slot, wheel_mode_name,
};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use transport::{C989, list_interfaces, open_all_interfaces};

#[derive(Parser)]
#[command(
    name = "c989ctl",
    version,
    about = "Native Linux control utility for the Cloud Nine C989 ErgoFSK keyboard"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
enum Layer {
    Base,
    Fn,
}

#[derive(Clone, Copy, ValueEnum, PartialEq, Eq)]
enum BlockFilter {
    Base,
    Fn,
    Rgb,
    Wheel,
}

#[derive(Subcommand)]
enum Commands {
    /// Set RGB lighting mode, color, brightness, speed, or per-key LEDs (`04 13`)
    Rgb {
        /// Lighting mode (e.g. static, breath, volcano, pinwheel, user-define, or 0x01..0x13/0x80).
        /// Defaults to `user-define` if `--key` is given, otherwise `static`.
        #[arg(short, long)]
        mode: Option<String>,

        /// Color (red, green, blue, white, off, orange, purple, or RRGGBB)
        #[arg(short, long, default_value = "ff0000")]
        color: String,

        /// Brightness level (0..=6; 0-1 off, 2-5 steps, 5-6 max)
        #[arg(short, long, default_value_t = 6)]
        brightness: u8,

        /// Animation speed (1..=10; 1 = static/paused, 10 = fastest)
        #[arg(short, long, default_value_t = 6)]
        speed: u8,

        /// Override full-spectrum / rainbow flag (byte 8)
        #[arg(long)]
        full_spectrum: Option<bool>,

        /// Set individual key LEDs in user-define (0x80) mode: KEY=COLOR (repeatable)
        #[arg(short = 'k', long = "key", value_name = "KEY=COLOR")]
        keys: Vec<String>,
    },

    /// Remap a single key on the base (`04 11`) or Fn (`04 27`) layer (with 15s auto-revert safety net)
    Remap {
        /// Physical key name (e.g. insert, m1, left_space; see `list-keys`)
        key: String,

        /// Action (e.g. a, ctrl+c, mouse_left, scroll_up, play_pause, disabled, default, macro:1)
        action: String,

        /// Target keymap layer
        #[arg(short, long, value_enum, default_value_t = Layer::Base)]
        layer: Layer,

        /// Optional base profile TOML to start from (defaults to built-in verified baseline)
        #[arg(short, long)]
        profile: Option<PathBuf>,

        /// Skip the 15-second confirmation prompt and auto-revert safety net
        #[arg(short = 'y', long = "yes", alias = "no-revert")]
        yes: bool,

        /// Auto-revert timeout in seconds
        #[arg(long, default_value_t = 15)]
        timeout: u64,
    },

    /// Configure the Smart Wheel (`04 25`, 4 mode/color slots, no `04 f0` apply)
    Wheel {
        /// Set a slot (1..=4) in format SLOT:MODE:COLOR (e.g. 1:volume:0000ff) (repeatable)
        #[arg(short = 's', long = "set", value_name = "SLOT:MODE:COLOR")]
        sets: Vec<String>,

        /// Reset all 4 Smart Wheel slots to the captured baseline defaults
        #[arg(long)]
        reset: bool,
    },

    /// Apply a TOML profile (keymap, RGB, Smart Wheel) with 15s keymap auto-revert by default
    Apply {
        /// Path to the profile .toml file
        profile: PathBuf,

        /// Apply only a specific block from the profile
        #[arg(long, value_enum)]
        only: Option<BlockFilter>,

        /// Optional profile to revert keymaps to if not confirmed (defaults to verified baseline)
        #[arg(long)]
        revert_to: Option<PathBuf>,

        /// Skip the 15-second confirmation prompt and auto-revert safety net
        #[arg(short = 'y', long = "yes", alias = "no-revert")]
        yes: bool,

        /// Auto-revert timeout in seconds
        #[arg(long, default_value_t = 15)]
        timeout: u64,
    },

    /// Flash your exact verified safe baseline (base keymap, Fn keymap, Smart Wheel)
    RestoreBaseline {
        /// Also restore baseline static blue RGB lighting
        #[arg(long)]
        include_rgb: bool,
    },

    /// Print all 115 physical keys, indices, and default HID codes
    ListKeys,

    /// Print all supported RGB modes and Smart Wheel modes
    ListModes,

    /// Print the verified baseline profile in TOML format
    DumpBaseline,

    /// Run background daemon listening for `macro:<slot>` HID triggers (app launcher & macros)
    Daemon {
        /// Path to the profile .toml file containing `[actions.<slot>]` definitions
        profile: PathBuf,
    },

    /// List C989 HID interfaces and monitor raw HID input reports (useful for probing macros/triggers)
    Listen {
        /// How many seconds to listen for HID input reports
        #[arg(short, long, default_value_t = 15)]
        duration: u64,

        /// Temporarily remap key(s) to macro slot(s) (KEY:SLOT, e.g. m6:1), listen, then auto-revert (repeatable)
        #[arg(long, value_name = "KEY:SLOT")]
        test_macro: Vec<String>,

        /// Also dump raw USB HID report descriptors from sysfs for each unique hidraw device
        #[arg(long)]
        dump_descriptors: bool,
    },
}

fn confirm_or_revert<F>(timeout_secs: u64, description: &str, revert: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    println!("Applied {description}.");
    print!(
        "Keep this change? Type 'y' + Enter within {timeout_secs}s (anything else or timeout auto-reverts): "
    );
    io::stdout().flush().ok();

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let _ = tx.send(input.trim().to_lowercase());
        }
    });

    match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
        Ok(ans) if ans == "y" || ans == "yes" => {
            println!("Change kept.");
            Ok(())
        }
        Ok(_) => {
            println!("Reverting...");
            revert()?;
            println!("Reverted cleanly!");
            Ok(())
        }
        Err(_) => {
            println!("\nTimed out after {timeout_secs}s — reverting to safe keymap...");
            revert()?;
            println!("Reverted cleanly!");
            Ok(())
        }
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ListKeys => {
            println!("{:<14} {:>5} {:>5} {:>6}  STATUS    ALIASES", "NAME", "KEY", "LED", "HID");
            for k in KEYS {
                let status = if k.remappable { "" } else { "[locked]" };
                println!(
                    "{:<14} {:>5} {:>5}   0x{:02x}  {:<8}  {}",
                    k.name,
                    k.key_index,
                    k.light_index,
                    k.default_code,
                    status,
                    k.aliases.join(", ")
                );
            }
            return Ok(());
        }
        Commands::ListModes => {
            println!("RGB Modes (04 13):");
            for m in ALL_RGB_MODES {
                println!(
                    "  0x{:02x}  {:<12}  (default full_spectrum: {})",
                    m.id(),
                    m.name(),
                    m.default_full_spectrum()
                );
            }
            println!("\nSmart Wheel Modes (04 25):");
            for id in 0u8..=5 {
                println!("  0x{id:02x}  {}", wheel_mode_name(id));
            }
            return Ok(());
        }
        Commands::DumpBaseline => {
            let toml_str = Profile::baseline().to_toml_string()?;
            print!("{toml_str}");
            return Ok(());
        }
        Commands::Daemon { profile } => {
            let prof = Profile::load(&profile)?;
            let actions = prof.parsed_actions()?;
            return daemon::run_daemon(&prof.name, actions);
        }
        Commands::Listen {
            duration,
            test_macro,
            dump_descriptors,
        } => {
            let ifaces = list_interfaces()?;
            if ifaces.is_empty() {
                return Err("No C989 (0c45:8006) HID interfaces found.".into());
            }
            println!("Detected C989 HID collections:");
            for i in &ifaces {
                println!(
                    "  iface={}  usage_page=0x{:04x}  usage=0x{:04x}  path={}",
                    i.interface_number, i.usage_page, i.usage, i.path
                );
            }

            let revert_base = if !test_macro.is_empty() {
                let baseline = baseline_base_keymap();
                let mut test_map = baseline;
                println!("\nTemporarily applying test macro mapping(s) for {duration}s:");
                for spec in &test_macro {
                    let (k_str, slot_str) = spec
                        .split_once(':')
                        .ok_or_else(|| format!("expected KEY:SLOT (e.g. m6:1), got '{spec}'"))?;
                    let info = find_key(k_str)
                        .ok_or_else(|| format!("unknown physical key '{k_str}'"))?;
                    if !info.remappable {
                        return Err(format!("key '{}' is hardware-locked", info.name));
                    }
                    let entry = KeyEntry::parse(&format!("macro:{slot_str}"))?;
                    set_keymap_slot(&mut test_map, info.key_index, entry);
                    println!(
                        "  {} (slot {}) -> macro:{slot_str} ({:02x?})",
                        info.name,
                        info.key_index,
                        entry.to_bytes()
                    );
                }

                let kb = C989::open()?;
                kb.write_base_keymap(&test_map)?;
                Some((kb, baseline))
            } else {
                None
            };

            let handles = open_all_interfaces()?;
            if dump_descriptors {
                println!("\nHID Report Descriptors:");
                for h in &handles {
                    if let Some(dev_name) = h.path.rsplit('/').next() {
                        let sys_path = format!("/sys/class/hidraw/{dev_name}/device/report_descriptor");
                        match std::fs::read(&sys_path) {
                            Ok(bytes) => {
                                let hex: String = bytes
                                    .iter()
                                    .map(|b| format!("{b:02x}"))
                                    .collect::<Vec<_>>()
                                    .join(" ");
                                println!(
                                    "  iface={} ({}) [{} bytes]:\n    {hex}",
                                    h.interface_number,
                                    h.path,
                                    bytes.len()
                                );
                            }
                            Err(e) => {
                                println!("  iface={} ({}): could not read {sys_path}: {e}", h.interface_number, h.path);
                            }
                        }
                    }
                }
            }

            println!(
                "\nListening on {} unique hidraw device(s) for {duration}s... Press your test key(s) now!",
                handles.len()
            );
            let start = Instant::now();
            let limit = Duration::from_secs(duration);
            let mut buf = [0u8; 64];

            while start.elapsed() < limit {
                let mut got_any = false;
                for h in &handles {
                    match h.dev.read(&mut buf) {
                        Ok(n) if n > 0 => {
                            got_any = true;
                            let hex: String = buf[..n]
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<Vec<_>>()
                                .join(" ");
                            println!(
                                "[{:6.2}s] iface={} ({}) len={:2}: {hex}",
                                start.elapsed().as_secs_f32(),
                                h.interface_number,
                                h.path,
                                n
                            );
                        }
                        _ => {}
                    }
                }
                if !got_any {
                    thread::sleep(Duration::from_millis(5));
                }
            }

            if let Some((kb, baseline)) = revert_base {
                println!("\nAuto-reverting base keymap to baseline...");
                kb.write_base_keymap(&baseline)?;
                println!("Reverted cleanly!");
            }
            return Ok(());
        }
        _ => {}
    }

    let kb = C989::open()?;

    match cli.command {
        Commands::Rgb {
            mode,
            color,
            brightness,
            speed,
            full_spectrum,
            keys,
        } => {
            let rgb_mode = match mode {
                Some(m) => RgbMode::parse(&m)?,
                None if !keys.is_empty() => RgbMode::UserDefine,
                None => RgbMode::Static,
            };
            let rgb_color = parse_color(&color)?;
            let full_spec = full_spectrum.unwrap_or_else(|| rgb_mode.default_full_spectrum());

            let mut per_key = BTreeMap::new();
            for spec in &keys {
                let (k_str, c_str) = spec
                    .split_once('=')
                    .ok_or_else(|| format!("expected KEY=COLOR, got '{spec}'"))?;
                let info = find_key(k_str)
                    .ok_or_else(|| format!("unknown physical key '{k_str}'"))?;
                let c = parse_color(c_str)?;
                per_key.insert(info.light_index, c);
            }

            let cfg = RgbConfig {
                mode: rgb_mode,
                color: rgb_color,
                full_spectrum: full_spec,
                brightness,
                speed,
                per_key: per_key.clone(),
            };
            let pages = build_rgb_pages(&cfg);
            kb.write_rgb(&pages)?;

            if rgb_mode == RgbMode::UserDefine {
                println!(
                    "Set RGB mode={} brightness={} with {} lit key(s).",
                    rgb_mode,
                    brightness,
                    per_key.len()
                );
            } else {
                println!(
                    "Set RGB mode={} color={} brightness={} speed={} full_spectrum={}",
                    rgb_mode,
                    format_hex_color(rgb_color),
                    brightness,
                    speed,
                    full_spec
                );
            }
        }

        Commands::Remap {
            key,
            action,
            layer,
            profile,
            yes,
            timeout,
        } => {
            let info = find_key(&key)
                .ok_or_else(|| format!("unknown physical key '{key}' (run `c989ctl list-keys`)"))?;
            if !info.remappable {
                return Err(format!(
                    "key '{}' (slot {}) is hardware-locked and cannot be remapped",
                    info.name, info.key_index
                ));
            }
            let entry = KeyEntry::parse(&action)?;

            let revert_pages = match profile {
                Some(path) => {
                    let compiled = Profile::load(&path)?.compile()?;
                    match layer {
                        Layer::Base => compiled.base_keymap.unwrap_or_else(baseline_base_keymap),
                        Layer::Fn => compiled.fn_keymap.unwrap_or_else(baseline_fn_keymap),
                    }
                }
                None => match layer {
                    Layer::Base => baseline_base_keymap(),
                    Layer::Fn => baseline_fn_keymap(),
                },
            };

            let mut new_pages = revert_pages;
            set_keymap_slot(&mut new_pages, info.key_index, entry);

            let layer_name = match layer {
                Layer::Base => "base (04 11)",
                Layer::Fn => "Fn (04 27)",
            };
            println!(
                "Flashing {layer_name} remap: {} (slot {}) -> {} ({:02x?})...",
                info.name,
                info.key_index,
                action,
                entry.to_bytes()
            );

            match layer {
                Layer::Base => kb.write_base_keymap(&new_pages)?,
                Layer::Fn => kb.write_fn_keymap(&new_pages)?,
            }

            if yes {
                println!("Remap applied (--yes).");
            } else {
                let desc = format!("{layer_name} remap ({} -> {action})", info.name);
                confirm_or_revert(timeout, &desc, || match layer {
                    Layer::Base => kb.write_base_keymap(&revert_pages),
                    Layer::Fn => kb.write_fn_keymap(&revert_pages),
                })?;
            }
        }

        Commands::Wheel { sets, reset } => {
            if !reset && sets.is_empty() {
                return Err(
                    "specify at least one `--set SLOT:MODE:COLOR` (e.g. `--set 1:volume:0000ff`) or `--reset`"
                        .into(),
                );
            }
            let mut slots = baseline_smart_wheel_slots();
            if !reset {
                for spec in &sets {
                    let parts: Vec<&str> = spec.split(':').collect();
                    if parts.len() != 3 {
                        return Err(format!(
                            "expected SLOT:MODE:COLOR (e.g. 1:volume:0000ff), got '{spec}'"
                        ));
                    }
                    let slot_num: usize = parts[0]
                        .parse()
                        .map_err(|_| format!("invalid slot number '{}' (expected 1..=4)", parts[0]))?;
                    if !(1..=4).contains(&slot_num) {
                        return Err(format!("slot number must be 1..=4, got {slot_num}"));
                    }
                    let mode = parse_wheel_mode(parts[1])?;
                    let color = parse_color(parts[2])?;
                    slots[slot_num - 1] = protocol::SmartWheelSlot { mode, color };
                }
            }

            let page = build_smart_wheel_page(&slots);
            kb.write_smart_wheel(&page)?;
            println!("Updated Smart Wheel (04 25, no 04 f0):");
            for (i, s) in slots.iter().enumerate() {
                println!(
                    "  Slot {}: {} (0x{:02x})  {}",
                    i + 1,
                    wheel_mode_name(s.mode),
                    s.mode,
                    format_hex_color(s.color)
                );
            }
        }

        Commands::Apply {
            profile,
            only,
            revert_to,
            yes,
            timeout,
        } => {
            let prof = Profile::load(&profile)?;
            let compiled = prof.compile()?;

            let do_base = (only.is_none() || only == Some(BlockFilter::Base))
                && compiled.base_keymap.is_some();
            let do_fn =
                (only.is_none() || only == Some(BlockFilter::Fn)) && compiled.fn_keymap.is_some();
            let do_rgb =
                (only.is_none() || only == Some(BlockFilter::Rgb)) && compiled.rgb.is_some();
            let do_wheel = (only.is_none() || only == Some(BlockFilter::Wheel))
                && compiled.smart_wheel.is_some();

            if !do_base && !do_fn && !do_rgb && !do_wheel {
                return Err(format!(
                    "profile '{}' has no matching sections to apply",
                    prof.name
                ));
            }

            let (revert_base, revert_fn) = match revert_to {
                Some(ref rpath) => {
                    let rc = Profile::load(rpath)?.compile()?;
                    (
                        rc.base_keymap.unwrap_or_else(baseline_base_keymap),
                        rc.fn_keymap.unwrap_or_else(baseline_fn_keymap),
                    )
                }
                None => (baseline_base_keymap(), baseline_fn_keymap()),
            };

            println!("Applying profile '{}'...", prof.name);
            if do_base {
                println!("  -> Writing base keymap (04 11)...");
                kb.write_base_keymap(compiled.base_keymap.as_ref().unwrap())?;
            }
            if do_fn {
                println!("  -> Writing Fn keymap (04 27)...");
                kb.write_fn_keymap(compiled.fn_keymap.as_ref().unwrap())?;
            }
            if do_wheel {
                println!("  -> Writing Smart Wheel (04 25)...");
                kb.write_smart_wheel(compiled.smart_wheel.as_ref().unwrap())?;
            }
            if do_rgb {
                println!("  -> Writing RGB lighting (04 13)...");
                kb.write_rgb(compiled.rgb.as_ref().unwrap())?;
            }

            if (do_base || do_fn) && !yes {
                let desc = format!("profile '{}'", prof.name);
                confirm_or_revert(timeout, &desc, || {
                    if do_base {
                        kb.write_base_keymap(&revert_base)?;
                    }
                    if do_fn {
                        kb.write_fn_keymap(&revert_fn)?;
                    }
                    Ok(())
                })?;
            } else {
                println!("Profile '{}' applied.", prof.name);
            }
        }

        Commands::RestoreBaseline { include_rgb } => {
            println!("Restoring verified safe baseline...");
            kb.write_base_keymap(&baseline_base_keymap())?;
            println!("  -> Base keymap (04 11) restored.");
            kb.write_fn_keymap(&baseline_fn_keymap())?;
            println!("  -> Fn keymap (04 27) restored.");
            kb.write_smart_wheel(&build_smart_wheel_page(&baseline_smart_wheel_slots()))?;
            println!("  -> Smart Wheel (04 25) restored.");
            if include_rgb {
                let compiled = Profile::baseline().compile()?;
                kb.write_rgb(compiled.rgb.as_ref().unwrap())?;
                println!("  -> RGB lighting (04 13) restored.");
            }
            println!("Baseline restore complete.");
        }

        Commands::ListKeys
        | Commands::ListModes
        | Commands::DumpBaseline
        | Commands::Daemon { .. }
        | Commands::Listen { .. } => {
            unreachable!()
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
