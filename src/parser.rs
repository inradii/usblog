use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use once_cell::sync::Lazy;
use regex::Regex;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::types::UsbEvent;

// Kernel log lines containing USB events have the form:
//   ...] usb PORT: message   or   ...: usb PORT: message
static RE_USB: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:\]|:) usb (.*?): ").unwrap());

static RE_VID: Lazy<Regex> = Lazy::new(|| Regex::new(r"idVendor=(\w+)").unwrap());
static RE_PID: Lazy<Regex> = Lazy::new(|| Regex::new(r"idProduct=(\w+)").unwrap());
static RE_PROD: Lazy<Regex> = Lazy::new(|| Regex::new(r"Product: (.*)$").unwrap());
static RE_MANUFACT: Lazy<Regex> = Lazy::new(|| Regex::new(r"Manufacturer: (.*)$").unwrap());
static RE_SERIAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"SerialNumber: (.*)$").unwrap());
// Matches "usb 1-1:" or "usb 2-3.4:" — the port identifier ends with a digit
static RE_PORT: Lazy<Regex> = Lazy::new(|| Regex::new(r"usb (.*[0-9]):").unwrap());

#[derive(Debug, Clone, PartialEq)]
enum Action {
    Connect,
    Disconnect,
}

#[derive(Debug, Clone)]
struct FilteredLine {
    date: String,
    action: Action,
    logline: String,
}

// Try to parse the leading timestamp and return (formatted_date, rest_of_line).
//
// Two formats are tried in order:
//   1. Journalctl ISO-8601: "2022-11-15T10:00:00.123456+00:00 ..." (32 chars)
//   2. Classic syslog:      "Nov 15 10:00:00 ..."                  (15 chars)
//
// Syslog entries get a "????" year placeholder because the year is absent.
fn parse_ts(line: &str) -> Option<(String, String)> {
    // --- ISO journalctl ---
    if line.len() >= 33 {
        let ts = line[..32].trim();
        // The timezone offset "+00:00" contains an extra colon; strip it so
        // chrono's %z directive (expecting "+0000") can parse it.
        let norm: String = if ts.matches(':').count() == 3 {
            match ts.rfind(':') {
                Some(i) => format!("{}{}", &ts[..i], &ts[i + 1..]),
                None => ts.to_string(),
            }
        } else {
            ts.to_string()
        };
        if let Ok(dt) = chrono::DateTime::parse_from_str(&norm, "%Y-%m-%dT%H:%M:%S%.f%z") {
            return Some((
                dt.format("%Y-%m-%d %H:%M:%S").to_string(),
                line[32..].trim().to_string(),
            ));
        }
    }

    // --- Classic syslog
    if line.len() >= 16 {
        let ts = line[..15].trim();
        // Single-digit days appear as "Nov  5"; normalise to "Nov 05".
        let norm = if ts.contains("  ") {
            ts.replacen("  ", " 0", 1)
        } else {
            ts.to_string()
        };
        // Prepend a dummy year so NaiveDateTime can parse the full format.
        let with_year = format!("2000 {}", norm);
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&with_year, "%Y %b %d %H:%M:%S") {
            return Some((
                format!("????-{}", dt.format("%m-%d %H:%M:%S")),
                line[15..].trim().to_string(),
            ));
        }
    }

    None
}

/// Process raw text and return filtered USB event lines with parsed timestamps and actions
fn process_text(text: &str) -> Vec<FilteredLine> {
    let mut out = Vec::new();
    for line in text.lines() {
        if !RE_USB.is_match(line) {
            continue;
        }
        let (date, logline) = match parse_ts(line) {
            Some(p) => p,
            None => continue,
        };
        let action = if line.contains("New USB device found, ")
            || line.contains("Product: ")
            || line.contains("Manufacturer: ")
            || line.contains("SerialNumber: ")
        {
            Action::Connect
        } else if line.contains("disconnect") {
            Action::Disconnect
        } else {
            continue;
        };
        out.push(FilteredLine {
            date,
            action,
            logline,
        });
    }
    out
}

/// Read the contents of a file, decompressing if necessary
fn read_file(path: &Path) -> Result<String> {
    if path.extension().and_then(|e| e.to_str()) == Some("gz") {
        let file =
            std::fs::File::open(path).with_context(|| format!("Cannot open {}", path.display()))?;
        let mut dec = GzDecoder::new(file);
        let mut s = String::new();
        dec.read_to_string(&mut s)
            .with_context(|| format!("Cannot decompress {}", path.display()))?;
        Ok(s)
    } else {
        std::fs::read_to_string(path).with_context(|| format!("Cannot read {}", path.display()))
    }
}

/// Find default log files for USB events
pub fn find_default_log_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for pattern in &["/var/log/syslog*", "/var/log/messages*"] {
        if let Ok(paths) = glob::glob(pattern) {
            for p in paths.filter_map(|r| r.ok()) {
                files.push(p);
            }
        }
    }
    // Sort oldest-first by modification time so events are in chronological order.
    files.sort_by_key(|p| {
        p.metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH)
    });
    files
}

/// Collect filtered USB event lines from a list of log files
fn collect_filtered(files: &[PathBuf], quiet: bool) -> Vec<FilteredLine> {
    let mut all = Vec::new();
    for path in files {
        if !quiet {
            eprintln!("Reading {}...", path.display());
        }
        match read_file(path) {
            Ok(text) => all.extend(process_text(&text)),
            Err(e) => eprintln!("Warning: {}", e),
        }
    }
    all
}

/// Collect filtered USB event lines from the system journal using journalctl
fn collect_journalctl(quiet: bool) -> Result<Vec<FilteredLine>> {
    if !quiet {
        eprintln!("Reading journalctl output...");
    }
    let out = Command::new("journalctl")
        .args(["-b", "0", "--no-pager"])
        .output()
        .context("Failed to run journalctl")?;
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(process_text(&text))
}

/// State-machine parser that mirrors usbrip's _parse_history logic:
///
///   link 1 -> waiting for "New USB device found"
///   link 2 -> next connect line should carry Product
///   link 3 -> next connect line should carry Manufacturer
///   link 4 -> next connect line may carry SerialNumber (optional)
///
/// Disconnect lines are matched back to the most recent event sharing the
/// same port.
fn parse_history(lines: Vec<FilteredLine>) -> Vec<UsbEvent> {
    let mut events: Vec<UsbEvent> = Vec::new();
    let mut curr: i64 = -1;
    let mut link: u8 = 1;
    let mut interrupted = false;

    for item in lines {
        match item.action {
            Action::Connect => {
                if item.logline.contains("New USB device found, ") {
                    let host = item.logline.splitn(2, ' ').next().unwrap_or("").to_string();
                    let vid = RE_VID
                        .captures(&item.logline)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string());
                    let pid = RE_PID
                        .captures(&item.logline)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string());
                    let port = RE_PORT
                        .captures(&item.logline)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str().to_string());

                    events.push(UsbEvent {
                        conn: Some(item.date),
                        host: Some(host),
                        vid,
                        pid,
                        port,
                        ..UsbEvent::default()
                    });
                    curr += 1;
                    link = 2;
                    interrupted = false;
                } else if !interrupted && curr >= 0 {
                    let idx = curr as usize;
                    match link {
                        2 => match RE_PROD.captures(&item.logline) {
                            Some(cap) => {
                                events[idx].prod =
                                    cap.get(1).map(|m| m.as_str().trim().to_string());
                                link = 3;
                            }
                            None => interrupted = true,
                        },
                        3 => match RE_MANUFACT.captures(&item.logline) {
                            Some(cap) => {
                                events[idx].manufact =
                                    cap.get(1).map(|m| m.as_str().trim().to_string());
                                link = 4;
                            }
                            None => interrupted = true,
                        },
                        4 => {
                            if let Some(cap) = RE_SERIAL.captures(&item.logline) {
                                events[idx].serial =
                                    cap.get(1).map(|m| m.as_str().trim().to_string());
                            }
                            interrupted = true;
                        }
                        _ => {}
                    }
                }
            }
            Action::Disconnect => {
                if let Some(port) = RE_PORT
                    .captures(&item.logline)
                    .and_then(|c| c.get(1))
                    .map(|m| m.as_str().to_string())
                {
                    // Walk backwards to find the most recent event on this port.
                    for e in events.iter_mut().rev() {
                        if e.port.as_deref() == Some(&port) {
                            e.disconn = Some(item.date.clone());
                            break;
                        }
                    }
                }
            }
        }
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ts_handles_syslog_and_journalctl_formats() {
        let (date, _) = parse_ts("Nov 15 10:00:00 host kernel: usb 1-1: msg").unwrap();
        assert!(date.starts_with("????-11-15 10:00:00"));

        let (date, _) =
            parse_ts("2022-11-15T10:00:00.123456+00:00 host kernel: usb 1-1: msg").unwrap();
        assert_eq!(date, "2022-11-15 10:00:00");
    }
}

/// Read and parse USB events from log files or the system journal
pub fn read_and_parse(
    files: &[PathBuf],
    use_journalctl: bool,
    quiet: bool,
) -> Result<Vec<UsbEvent>> {
    let lines = if use_journalctl {
        collect_journalctl(quiet)?
    } else {
        collect_filtered(files, quiet)
    };
    Ok(parse_history(lines))
}
