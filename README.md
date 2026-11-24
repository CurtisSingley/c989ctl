# c989ctl

Native Linux control utility (**CLI + native `iced` GUI**) for the **Cloud Nine C989 ErgoFSK** mechanical keyboard (`VID 0x0c45`, `PID 0x8006`, SONiX).

Built and verified on **Arch Linux + i3 (X11)**.

## Features

### 100% Onboard Hardware Control (No Background Process Required)
All of these settings are written directly to the C989's onboard flash and persist across reboots and unplugs:
- **Base Layer (`04 11`) & Fn Layer (`04 27`) Keymaps**:
  - Standard keys, modifier combos (`ctrl+c`, `lctrl+shift+t`), mouse buttons (`mouse_left`, `mouse_right`, `mouse_middle`), mouse scroll (`scroll_up`, `scroll_down`), consumer/media keys (`play_pause`, `mute`, `vol_up`, `vol_down`, `prev_track`, `next_track`, `stop`), key disable (`disabled`), and hardware-locked key protection (`m5`, `m10`, `rfn`).
  - **15-second auto-revert safety net** enabled by default on all keymap writes in both CLI and GUI.
- **RGB Lighting (`04 13`)**:
  - All 20 hardware lighting modes (`static` through `single_off` + `user_define` per-key RGB `0x80`).
  - Brightness (`0..=6`; `0–1` off, `5–6` max), animation speed (`1..=10`; `1` paused, `10` fastest), and `full_spectrum` rainbow toggle.
  - Per-key RGB lighting & visual painter across all 115 keys.
- **Smart Wheel (`04 25`)**:
  - All 4 wheel slots with mode (`volume`, `task_switcher`, `vertical_scroll`, `brightness`, `horizontal_scroll`, `zoom`) and RGB color.
- **TOML Profiles**:
  - Human-readable `.toml` profiles stored in `~/.config/c989ctl/profiles/` (portable between CLI and GUI).

### Optional Host-Side Macro & App-Launcher Listener (Off by Default)
The C989 firmware does not execute multi-step macros or launch applications onboard; keys mapped to `macro:<1..=255>` (`05 02 <slot> 00`) emit a clean 4-byte vendor HID report (`05 00 <slot> 00`) on Interface 1.
- The listener is **disabled by default** and strictly opt-in.
- Supports `launch` (detached apps), `shell`, `text` (literal typing), and multi-step `macro` sequences (`key`, `text`, `delay`, `exec`).

## Setup (Arch Linux)

### 1. udev Rule (Non-root HID Access)
Create `/etc/udev/rules.d/70-c989.rules` (must be numbered `< 73` so `73-seat-late.rules` applies `uaccess` ACLs):

```udev
SUBSYSTEM=="hidraw", ATTRS{idVendor}=="0c45", ATTRS{idProduct}=="8006", TAG+="uaccess"
```

Reload udev rules and replug the keyboard:
```bash
sudo udevadm control --reload-rules && sudo udevadm trigger
```

### 2. Dependencies
- **Build**: Rust toolchain (`rustup` / `cargo`), `hidapi` (`libudev` / `systemd-libs`).
- **Optional (only if enabling host text/key macros)**: `xdotool` (X11).

### 3. Build
```bash
# Build both CLI (`c989ctl`) and native GUI (`c989ctl-gui`)
cargo build --release

# Or build headless CLI-only (no GUI dependencies)
cargo build --release --no-default-features
```

## Usage

### Native GUI
```bash
./target/release/c989ctl-gui
```
- Click any key on the visual C989 layout to edit its **Base** or **Fn** binding.
- Switch to **Lighting** to pick effects, adjust brightness/speed, or left-click / right-click keys to paint per-key LEDs in `user_define` mode.
- Switch to **Smart Wheel** to edit all 4 wheel slots and colors.
- Switch to **Macros & Actions** to configure optional `[actions.<slot>]` entries and explicitly enable/disable the listener.

### CLI
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

# Profiles
c989ctl apply profiles/default.toml
c989ctl restore-baseline --include-rgb

# Optional host macro/launcher daemon (only needed for `macro:<slot>` actions)
c989ctl daemon profiles/macro_test.toml

# Discovery helpers
c989ctl list-keys
c989ctl list-modes
c989ctl dump-baseline
```

## Notes & Known Limitations on Other Setups
- **Prioritized for Arch Linux + i3 (X11)**:
  - Host-side `text` and `macro` keystrokes currently use `xdotool` (X11), which respects your active XKB layout (e.g., Dvorak). On **Wayland**, `launch` and `shell` actions work, while `text`/`macro` playback will need a future `uinput` or `ydotool`/`wtype` backend.
  - Onboard hardware remaps output standard USB HID Usage IDs (QWERTY positions); your OS keymap (e.g., Dvorak or Caps→Esc) translates them normally.
- **Fn Switch Mode**: Momentary Fn mode is assumed (Toggle mode is not implemented).
- **Smart Wheel modes `0x06`/`0x07`**: Likely map to Windows Surface Dial (`Report ID 0x08`), which is not bound by default on Linux/i3.
