use crate::types::UsbEvent;
use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

enum DeviceEvent {
    Connected(String, Box<UsbEvent>),
    Disconnected(String),
}

/// Read a single sysfs attribute and return it as a trimmed string or None if it doesn't exist or is empty
fn read_sysfs(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Construct a UsbEvent from a sysfs device path, if it represents a USB device. Returns the device key and the event
fn event_from_sysfs(dev_path: &Path) -> Option<(String, UsbEvent)> {
    let uevent = std::fs::read_to_string(dev_path.join("uevent")).ok()?;
    if !uevent.contains("DEVTYPE=usb_device") {
        return None;
    }
    let key = dev_path.file_name()?.to_str()?.to_string();
    Some((
        key.clone(),
        UsbEvent {
            vid: read_sysfs(&dev_path.join("idVendor")),
            pid: read_sysfs(&dev_path.join("idProduct")),
            prod: read_sysfs(&dev_path.join("product")),
            manufact: read_sysfs(&dev_path.join("manufacturer")),
            serial: read_sysfs(&dev_path.join("serial")),
            port: Some(key),
            ..UsbEvent::default()
        },
    ))
}

/// Scan the currently connected USB devices and return a map from device key to UsbEvent
fn scan_connected() -> HashMap<String, UsbEvent> {
    let mut map = HashMap::new();
    let Ok(entries) = std::fs::read_dir("/sys/bus/usb/devices") else {
        return map;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        if let Some((k, v)) = event_from_sysfs(&entry.path()) {
            map.insert(k, v);
        }
    }
    map
}

/// Monitor USB device events in real time and send them through the provided channel
fn monitor(tx: mpsc::Sender<DeviceEvent>) -> Result<()> {
    let mut child = Command::new("udevadm")
        .args(["monitor", "--udev", "--subsystem-match=usb", "--property"])
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to start udevadm: {}", e))?;

    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);
    let mut props: HashMap<String, String> = HashMap::new();

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        if line.is_empty() {
            let action = props.get("ACTION").map(String::as_str).unwrap_or("");
            let devtype = props.get("DEVTYPE").map(String::as_str).unwrap_or("");
            let devpath = props.get("DEVPATH").cloned().unwrap_or_default();

            if devtype == "usb_device" && !devpath.is_empty() {
                let key = Path::new(&devpath)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                match action {
                    "add" => {
                        let sys_path = format!("/sys{devpath}");
                        let event = event_from_sysfs(Path::new(&sys_path))
                            .map(|(_, e)| e)
                            .unwrap_or_else(|| UsbEvent {
                                vid: props.get("ID_VENDOR_ID").cloned(),
                                pid: props.get("ID_MODEL_ID").cloned(),
                                prod: props.get("ID_MODEL").map(|s| s.replace('_', " ")),
                                manufact: props.get("ID_VENDOR").map(|s| s.replace('_', " ")),
                                serial: props.get("ID_SERIAL_SHORT").cloned(),
                                port: Some(key.clone()),
                                ..UsbEvent::default()
                            });
                        let _ = tx.send(DeviceEvent::Connected(key, Box::new(event)));
                    }
                    "remove" => {
                        let _ = tx.send(DeviceEvent::Disconnected(key));
                    }
                    _ => {}
                }
            }
            props.clear();
        } else if let Some((k, v)) = line.split_once('=') {
            props.insert(k.to_string(), v.to_string());
        }
    }

    Ok(())
}

/// Format an optional string for display, returning "-" if None
fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("-")
}

/// Draw the current state of connected USB devices to the terminal
fn is_ignored(e: &UsbEvent, ignore_vid: &[String], ignore_pid: &[String]) -> bool {
    let vid_match = |v: &str| ignore_vid.iter().any(|f| v.eq_ignore_ascii_case(f));
    let pid_match = |v: &str| ignore_pid.iter().any(|f| v.eq_ignore_ascii_case(f));
    e.vid.as_deref().is_some_and(vid_match) || e.pid.as_deref().is_some_and(pid_match)
}

fn draw(devices: &HashMap<String, UsbEvent>, ignore_vid: &[String], ignore_pid: &[String]) {
    print!("\x1b[2J\x1b[H");
    println!("\n{}", "USB Live Monitor  (Ctrl-C to exit)".bold());
    println!("{}", "─".repeat(76).dimmed());

    if devices.is_empty() {
        println!("{}", "  No USB devices currently connected.".dimmed());
    } else {
        println!(
            "  {:<8} {:<8} {:<24} {:<20} {}",
            "VID".bold(),
            "PID".bold(),
            "Product".bold(),
            "Manufacturer".bold(),
            "Port".bold(),
        );
        println!("{}", "─".repeat(76).dimmed());

        let mut rows: Vec<&UsbEvent> = devices
            .values()
            .filter(|e| !is_ignored(e, ignore_vid, ignore_pid))
            .collect();
        rows.sort_by_key(|e| e.port.as_deref().unwrap_or(""));

        for e in rows {
            println!(
                "  {:<8} {:<8} {:<24} {:<20} {}",
                opt(&e.vid).cyan(),
                opt(&e.pid).cyan(),
                opt(&e.prod),
                opt(&e.manufact),
                opt(&e.port).dimmed(),
            );
        }
    }

    println!();
    let shown = devices
        .values()
        .filter(|e| !is_ignored(e, ignore_vid, ignore_pid))
        .count();
    println!("{}", format!("  {} device(s) connected", shown).green());
}

/// Entry point for the live USB monitor
pub fn run(ignore_vid: &[String], ignore_pid: &[String]) -> Result<()> {
    let (tx, rx) = mpsc::channel::<DeviceEvent>();

    let mut devices = scan_connected();
    draw(&devices, ignore_vid, ignore_pid);

    thread::spawn(move || {
        let _ = monitor(tx);
    });

    for event in rx {
        match event {
            DeviceEvent::Connected(key, device) => {
                devices.insert(key, *device);
            }
            DeviceEvent::Disconnected(key) => {
                devices.remove(&key);
            }
        }
        draw(&devices, ignore_vid, ignore_pid);
    }

    Ok(())
}
