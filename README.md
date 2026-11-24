# c989ctl

Native Linux control utility (**CLI + native `iced` GUI**) for the **Cloud Nine C989 ErgoFSK** ergonomic mechanical keyboard (`VID 0x0c45`, `PID 0x8006`, SONiX).

## Verified Environment
- **Keyboard Hardware**: Cloud Nine C989 ErgoFSK (`0c45:8006`)
- **Keyboard Firmware**: `1.82.0.0`
- **OS & Environment**: **Arch Linux**, **i3 (X11)**, **`dmenu`**, optional **`xdotool`** (for host-side text/macro playback)

> Other firmware versions, distros, and Wayland compositors have not been tested yet — see [Notes & Known Limitations](#notes--known-limitations) below.

## Features

### Onboard Hardware Control (No Background Process Required)
All of these settings are written directly to the C989's onboard flash and persist across reboots and unplugs:
- **Base Layer (`04 11`) & Fn Layer (`04 27`) Keymaps**:
  - Standard keys, modifier combos (`ctrl+c`, `lctrl+shift+t`), mouse buttons (`mouse_left`, `mouse_right`, `mouse_middle`), mouse scroll (`scroll_up`, `scroll_down`), consumer/media keys (`play_pause`, `mute`, `vol_up`, `vol_down`, `prev_track`, `next_track`, `stop`), key disable (`disabled`), and hardware-locked key protection (`m5`, `m10`, `rfn`).
  - **15-second auto-revert safety net** enabled by default on all keymap writes in both CLI and GUI.
- **RGB Lighting (`04 13`)**:
  - All 20 hardware lighting modes (`static` through `single_off` + `user_define` per-key RGB `0x80`).
  - Brightness (`0..=6`; `0–1` off, `5–6` max), animation speed (`1..=10`; `1` paused, `10` fastest), and `full_spectrum` rainbow toggle.
  - Interactive per-key RGB painter across all 115 keys.
- **Smart Wheel (`04 25`)**:
  - All 4 wheel slots with mode (`volume`, `task_switcher`, `vertical_scroll`, `brightness`, `horizontal_scroll`, `zoom`) and RGB color.
- **TOML Profiles**:
  - Human-readable `.toml` profiles stored in `~/.config/c989ctl/profiles/` (shared between CLI and GUI).

### Optional Host-Side Macro & App-Launcher Listener (Off by Default)
The C989 firmware does not execute multi-step macros or launch applications onboard; keys mapped to `macro:<1..=255>` (`05 02 <slot> 00`) emit a 4-byte vendor HID report (`05 00 <slot> 00`) on Interface 1.
- Strictly **opt-in and disabled by default** (can be toggled on inside `c989ctl-gui`, run manually via `c989ctl daemon`, or enabled via the included `systemd --user` service).
- Supports `launch` (detached apps), `shell`, `text` (literal typing), and multi-step `macro` sequences (`key`, `text`, `delay`, `exec`).

## Installation (Arch Linux)

### From AUR (once published)
```bash
paru -S c989ctl
```
After installing, unplug and replug your keyboard so `/usr/lib/udev/rules.d/70-c989.rules` grants non-root HID access. Both `c989ctl` and `c989ctl-gui` are installed to `/usr/bin/` and immediately launchable from **`dmenu`** or desktop launchers.

### Building from Source
1. Install dependencies:
   - **Build**: `rustup` / `cargo`, `systemd-libs` (`libudev`), `fontconfig`, `libxkbcommon`
   - **Optional (only for host text/macro playback on X11)**: `xdotool`
2. Build:
   ```bash
   # Build both CLI (`c989ctl`) and native GUI (`c989ctl-gui`)
   cargo build --release

   # Or build headless CLI-only (no GUI dependencies)
   cargo build --release --no-default-features
   ```
3. Install the udev rule (numbered `< 73` so `73-seat-late.rules` applies `uaccess` ACLs):
   ```bash
   sudo install -Dm644 packaging/70-c989.rules /etc/udev/rules.d/70-c989.rules
   sudo udevadm control --reload-rules && sudo udevadm trigger
   ```
   Unplug and replug the keyboard if permissions do not apply immediately.

### Installed Paths (Arch Package Layout)
| Asset | Path |
|---|---|
| CLI & GUI binaries | `/usr/bin/c989ctl`, `/usr/bin/c989ctl-gui` |
| udev rule | `/usr/lib/udev/rules.d/70-c989.rules` |
| Desktop entry | `/usr/share/applications/c989ctl.desktop` |
| Application icon | `/usr/share/icons/hicolor/scalable/apps/c989ctl.svg` |
| System default profile | `/usr/share/c989ctl/profiles/default.toml` |
| Optional user daemon unit | `/usr/lib/systemd/user/c989ctl-daemon.service` |
| User profiles (XDG) | `~/.config/c989ctl/profiles/*.toml` |

## Usage

### Native GUI (`c989ctl-gui`)
Launch `c989ctl-gui` via `dmenu`, your application launcher, or terminal:
- **Layout**: Toggle **Base Layer** vs. **Fn Layer** (highlighted with a purple border & badge), click any key to inspect or remap, and apply with the 15-second auto-revert safety net.
- **Lighting**: Choose any of the 20 effects, adjust brightness/speed, or switch to `user_define (0x80)` and left-click / right-click keys to paint individual LEDs.
- **Smart Wheel**: Configure all 4 wheel slots and colors.
- **Profiles**: Create, duplicate, save, delete (with confirmation), or right-click a profile in the sidebar to rename it.
- **Macros & Actions**: Configure `[actions.<slot>]` entries and optionally enable the host listener.

### CLI (`c989ctl`)
```bash
# RGB lighting
c989ctl rgb --mode static --color 0000ff --brightness 5
c989ctl rgb --mode volcano --full-spectrum true --brightness 6 --speed 8
c989ctl rgb --key esc=ff0000 --key insert=00ff00 --brightness 5

# Remap a key (15s auto-revert safety net by default; use --yes to skip)
c989ctl remap m6 shift+a
c989ctl remap --layer fn insert a

# Smart Wheel
c989ctl wheel --set 1:horizontal_scroll:ff00ff --set 2:zoom:00ffff
c989ctl wheel --reset

# Profiles & factory defaults
c989ctl apply ~/.config/c989ctl/profiles/default.toml
c989ctl restore-defaults --include-rgb

# Optional host macro/launcher daemon (off by default)
c989ctl daemon ~/.config/c989ctl/profiles/default.toml
# Or enable at login via systemd user unit:
systemctl --user enable --now c989ctl-daemon.service

# Discovery helpers
c989ctl list-keys
c989ctl list-modes
c989ctl dump-defaults
```

## Notes & Known Limitations
- **Tested Setup**: Built and verified on **Arch Linux + i3 (X11)** with C989 **Firmware `1.82.0.0`**. Other firmware revisions may use different packet layouts or offsets.
- **Wayland**: Onboard hardware control (keymaps, RGB, Smart Wheel, profiles) and `launch`/`shell` host actions work independently of X11, while host-side `text` and `macro` keystrokes currently rely on **`xdotool`** (X11) and are not yet supported on Wayland.
- **OS Keymaps (e.g., Dvorak / Caps→Esc)**: Onboard hardware remaps emit standard USB HID Usage IDs (QWERTY positions) that your OS XKB keymap translates normally. Host-side `xdotool` text macros respect your active X11 layout.
- **Factory Macro Key Defaults**: Leaving `m1`–`m4` or `m6`–`m9` unset (`00 00 00 00`) uses the C989's built-in shortcuts (`Ctrl+A/C/V/X` and `Ctrl+1..4`). Set a key to `disabled` if you want it to do nothing.
- **Fn Switch Mode**: Momentary Fn mode is assumed (Toggle mode is not implemented).
- **Smart Wheel modes `0x06`/`0x07`**: Map to Windows Surface Dial (`Report ID 0x08`), which is not bound by default on Linux/i3.

## License
MIT — see [LICENSE](LICENSE).
