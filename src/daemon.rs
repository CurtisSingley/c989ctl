use crate::profile::{ActionSpec, MacroStep};
use crate::transport::open_vendor_interface;
use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Decode a vendor macro/action trigger packet from Interface 1 (`/dev/hidraw*`).
/// Verified format: Report ID `0x05`, payload `[0x05, 0x00, <slot>, 0x00]`.
pub fn decode_vendor_trigger(report: &[u8]) -> Option<u8> {
    if report.len() >= 4 && report[0] == 0x05 && report[1] == 0x00 && report[3] == 0x00 && report[2] != 0 {
        Some(report[2])
    } else {
        None
    }
}

fn spawn_detached(program: &str, args: &[String]) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to launch '{program}': {e}"))?;

    // Reap the child in a background thread so we never leave zombies
    thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn run_xdotool_type(text: &str, delay_ms: u32) -> Result<(), String> {
    let status = Command::new("xdotool")
        .args([
            "type",
            "--clearmodifiers",
            "--delay",
            &delay_ms.to_string(),
            "--",
            text,
        ])
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("failed to run xdotool (is xdotool installed?): {e}"))?;
    if !status.success() {
        return Err(format!("xdotool type exited with {status}"));
    }
    Ok(())
}

fn run_xdotool_key(key: &str) -> Result<(), String> {
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", key])
        .stdin(Stdio::null())
        .status()
        .map_err(|e| format!("failed to run xdotool (is xdotool installed?): {e}"))?;
    if !status.success() {
        return Err(format!("xdotool key '{key}' exited with {status}"));
    }
    Ok(())
}

pub fn execute_action(slot: u8, action: &ActionSpec) -> Result<(), String> {
    match action {
        ActionSpec::Launch { command, args } => {
            println!("[slot {slot}] launching: {command} {}", args.join(" "));
            spawn_detached(command, args)
        }
        ActionSpec::Shell { command } => {
            println!("[slot {slot}] shell: {command}");
            spawn_detached("sh", &["-c".to_string(), command.clone()])
        }
        ActionSpec::Text { text, delay_ms } => {
            println!("[slot {slot}] typing text ({} chars)", text.len());
            // Brief pause so the physical trigger key release doesn't interfere with modifiers
            thread::sleep(Duration::from_millis(30));
            run_xdotool_type(text, *delay_ms)
        }
        ActionSpec::Macro { steps, delay_ms } => {
            println!("[slot {slot}] running macro ({} step(s))", steps.len());
            thread::sleep(Duration::from_millis(30));
            for step in steps {
                match step {
                    MacroStep::Key { key } => run_xdotool_key(key)?,
                    MacroStep::Text { text } => run_xdotool_type(text, *delay_ms)?,
                    MacroStep::Delay { delay_ms } => {
                        thread::sleep(Duration::from_millis(*delay_ms));
                    }
                    MacroStep::Exec { exec, args } => spawn_detached(exec, args)?,
                }
            }
            Ok(())
        }
    }
}

pub fn run_daemon(profile_name: &str, actions: BTreeMap<u8, ActionSpec>) -> Result<(), String> {
    let actions = Arc::new(actions);
    println!(
        "c989ctl daemon started for profile '{}' ({} action slot(s) configured):",
        profile_name,
        actions.len()
    );
    for (slot, act) in actions.iter() {
        match act {
            ActionSpec::Launch { command, args } => {
                println!("  slot {slot:3}: launch `{command} {}`", args.join(" "));
            }
            ActionSpec::Shell { command } => {
                println!("  slot {slot:3}: shell `{command}`");
            }
            ActionSpec::Text { text, .. } => {
                println!("  slot {slot:3}: text {:?}", text);
            }
            ActionSpec::Macro { steps, .. } => {
                println!("  slot {slot:3}: macro ({} steps)", steps.len());
            }
        }
    }
    println!("Listening for C989 vendor trigger reports (Ctrl+C to stop)...");

    let mut buf = [0u8; 64];
    loop {
        let handle = match open_vendor_interface() {
            Ok(h) => {
                println!("Connected to C989 interface {} ({}).", h.interface_number, h.path);
                h
            }
            Err(e) => {
                eprintln!("Waiting for C989 keyboard ({e})...");
                thread::sleep(Duration::from_secs(2));
                continue;
            }
        };

        loop {
            match handle.dev.read_timeout(&mut buf, 250) {
                Ok(0) => continue,
                Ok(n) => {
                    if let Some(slot) = decode_vendor_trigger(&buf[..n]) {
                        if let Some(action) = actions.get(&slot).cloned() {
                            thread::spawn(move || {
                                if let Err(err) = execute_action(slot, &action) {
                                    eprintln!("[slot {slot}] error: {err}");
                                }
                            });
                        } else {
                            println!("[slot {slot}] received trigger (no action configured)");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("C989 disconnected ({e}); waiting to reconnect...");
                    thread::sleep(Duration::from_secs(1));
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_verified_vendor_trigger_packets() {
        assert_eq!(decode_vendor_trigger(&[0x05, 0x00, 0x01, 0x00]), Some(1));
        assert_eq!(decode_vendor_trigger(&[0x05, 0x00, 0x0a, 0x00]), Some(10));
        assert_eq!(decode_vendor_trigger(&[0x05, 0x00, 0x05, 0x00]), Some(5));
        assert_eq!(decode_vendor_trigger(&[0x05, 0x00, 0x00, 0x00]), None);
        assert_eq!(decode_vendor_trigger(&[0x06, 0x00, 0x01, 0x00]), None);
        assert_eq!(decode_vendor_trigger(&[0x05, 0x00]), None);
    }
}
