use anyhow::Result;
use colored::Colorize;
use prettytable::{format, Cell, Row, Table};
use std::path::Path;

use crate::types::UsbEvent;

/// Helper to get the string value of an optional field, or "-" if None
fn opt(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("-")
}

/// Convert a USB event into a table row
fn event_row(e: &UsbEvent) -> Row {
    Row::new(vec![
        Cell::new(opt(&e.conn)),
        Cell::new(opt(&e.host)),
        Cell::new(opt(&e.vid)),
        Cell::new(opt(&e.pid)),
        Cell::new(opt(&e.prod)),
        Cell::new(opt(&e.manufact)),
        Cell::new(opt(&e.serial)),
        Cell::new(opt(&e.port)),
        Cell::new(opt(&e.disconn)),
    ])
}

/// Print USB events as a table
pub fn print_table(events: &[UsbEvent], violations: bool) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(Row::new(vec![
        Cell::new("Connected"),
        Cell::new("Host"),
        Cell::new("VID"),
        Cell::new("PID"),
        Cell::new("Product"),
        Cell::new("Manufacturer"),
        Cell::new("Serial Number"),
        Cell::new("Port"),
        Cell::new("Disconnected"),
    ]));
    for e in events {
        table.add_row(event_row(e));
    }
    table.printstd();
    summary(events.len(), violations);
}

/// Print USB events as a vertical list
pub fn print_list(events: &[UsbEvent], violations: bool) {
    for (i, e) in events.iter().enumerate() {
        println!("{}", format!("── Event #{} ", i + 1).dimmed());
        let pairs: &[(&str, &Option<String>)] = &[
            ("Connected",     &e.conn),
            ("Host",          &e.host),
            ("VID",           &e.vid),
            ("PID",           &e.pid),
            ("Product",       &e.prod),
            ("Manufacturer",  &e.manufact),
            ("Serial Number", &e.serial),
            ("Port",          &e.port),
            ("Disconnected",  &e.disconn),
        ];
        for (label, value) in pairs {
            let v = opt(value);
            let v_col = if violations {
                v.red().to_string()
            } else {
                v.cyan().to_string()
            };
            println!("  {:>15}: {}", label.bold(), v_col);
        }
        println!();
    }
    summary(events.len(), violations);
}

/// Print a summary of the number of events and whether there were violations
fn summary(n: usize, violations: bool) {
    let msg = format!("{} event(s) found.", n);
    if violations {
        println!("{}", msg.red().bold());
    } else {
        println!("{}", msg.green());
    }
}

/// Save USB events to a JSON file
pub fn save_json(events: &[UsbEvent], path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(events)?;
    std::fs::write(path, json)?;
    println!(
        "{}",
        format!("Saved {} event(s) to {}.", events.len(), path.display()).green()
    );
    Ok(())
}
