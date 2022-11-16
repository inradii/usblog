mod filter;
mod output;
mod parser;
mod types;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use colored::Colorize;
use std::collections::HashSet;
use std::path::PathBuf;

use filter::apply_filters;
use output::{print_list, print_table, save_json};
use parser::{find_default_log_files, read_and_parse};
use types::{FilterOpts, UsbEvent};

#[derive(Parser)]
#[clap(
    name = "usblog",
    about = "USB event collection tool for Linux",
    version
)]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Retrieve USB connection history from system logs
    History(HistoryArgs),

    /// Generate an authorized device list from current state
    Generate {
        /// Output JSON file
        output: PathBuf,
        /// Fields used as authorization keys: vid, pid, prod, manufact, serial, port, host
        #[clap(long = "key", default_values = &["vid", "pid"])]
        keys: Vec<String>,
        #[clap(flatten)]
        source: SourceArgs,
        #[clap(flatten)]
        filter: FilterArgs,
    },

    /// Find events absent from an authorized device list
    Violations {
        /// Path to the authorization JSON file
        auth: PathBuf,
        /// Fields to match against the authorization list
        #[clap(long = "key", default_values = &["vid", "pid"])]
        keys: Vec<String>,
        #[clap(flatten)]
        display: DisplayArgs,
        #[clap(flatten)]
        source: SourceArgs,
        #[clap(flatten)]
        filter: FilterArgs,
    },
}

// Arguments shared by commands that read raw log files
#[derive(Args, Default)]
struct SourceArgs {
    /// Log files to parse (default: /var/log/syslog* and /var/log/messages*)
    #[clap(short = 'f', long = "file")]
    files: Vec<PathBuf>,

    /// Read from journalctl instead of log files
    #[clap(short = 'j', long)]
    journalctl: bool,

    /// Suppress progress messages
    #[clap(short, long)]
    quiet: bool,
}

#[derive(Args, Default)]
struct HistoryArgs {
    #[clap(flatten)]
    display: DisplayArgs,
    #[clap(flatten)]
    source: SourceArgs,
    #[clap(flatten)]
    filter: FilterArgs,

    /// Save results to a JSON file
    #[clap(short = 'o', long)]
    output: Option<PathBuf>,
}

#[derive(Args, Default)]
struct DisplayArgs {
    /// Display as a table (default)
    #[clap(short = 't', long)]
    table: bool,
    /// Display as a vertical list
    #[clap(short = 'l', long)]
    list: bool,
}

#[derive(Args, Default)]
struct FilterArgs {
    /// Filter by date prefix, e.g., 2000-01-01
    #[clap(short = 'd', long)]
    date: Vec<String>,
    /// Filter by hostname
    #[clap(long)]
    host: Vec<String>,
    /// Filter by vendor ID
    #[clap(long)]
    vid: Vec<String>,
    /// Filter by product ID
    #[clap(long)]
    pid: Vec<String>,
    /// Filter by product name (substring)
    #[clap(long)]
    prod: Vec<String>,
    /// Filter by manufacturer (substring)
    #[clap(long)]
    manufact: Vec<String>,
    /// Filter by serial number
    #[clap(long)]
    serial: Vec<String>,
    /// Filter by port
    #[clap(long)]
    port: Vec<String>,
    /// Limit to N results
    #[clap(short = 'n')]
    num: Option<usize>,
}

impl From<FilterArgs> for FilterOpts {
    fn from(a: FilterArgs) -> FilterOpts {
        FilterOpts {
            date:     a.date,
            host:     a.host,
            vid:      a.vid,
            pid:      a.pid,
            prod:     a.prod,
            manufact: a.manufact,
            serial:   a.serial,
            port:     a.port,
            num:      a.num,
        }
    }
}

/// Print the banner for the tool
fn banner() {
    for line in &[
        "          _    _           ",
        "  _  _ __| |__| |___  __ _ ",
        " | || (_-< '_ \\ / _ \\/ _` |",
        "  \\_,_/__/_.__/_\\___/\\__, | by inradii technologies",
        "                     |___/ ",
    ] {
        println!("{}", line.cyan());
    }
    println!("{}\n", "USB Event collection and diff Tool for Linux".dimmed());
}

fn resolve_source(src: &SourceArgs) -> (Vec<PathBuf>, bool) {
    if !src.files.is_empty() {
        return (src.files.clone(), false);
    }
    if src.journalctl {
        return (vec![], true);
    }
    let defaults = find_default_log_files();
    let use_jctl = defaults.is_empty();
    (defaults, use_jctl)
}

fn display(events: &[UsbEvent], d: &DisplayArgs, violations: bool) {
    if d.list {
        print_list(events, violations);
    } else {
        print_table(events, violations);
    }
}

/// Entry point for the tool
fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command.unwrap_or_else(|| Commands::History(HistoryArgs::default())) {
        Commands::History(args) => {
            if !args.source.quiet {
                banner();
            }
            let (files, use_jctl) = resolve_source(&args.source);
            let events = read_and_parse(&files, use_jctl, args.source.quiet)?;
            let filtered = apply_filters(events, &FilterOpts::from(args.filter));
            display(&filtered, &args.display, false);
            if let Some(out) = args.output {
                save_json(&filtered, &out)?;
            }
        }

        Commands::Generate { output, keys, source, filter } => {
            if !source.quiet {
                banner();
            }
            let (files, use_jctl) = resolve_source(&source);
            let events = read_and_parse(&files, use_jctl, source.quiet)?;
            let events = apply_filters(events, &FilterOpts::from(filter));

            let mut seen: HashSet<Vec<Option<String>>> = HashSet::new();
            let mut entries: Vec<serde_json::Value> = Vec::new();

            for e in &events {
                let vals: Vec<Option<String>> = keys
                    .iter()
                    .map(|k| e.get_field(k).map(str::to_string))
                    .collect();
                if seen.insert(vals.clone()) {
                    let obj: serde_json::Map<String, serde_json::Value> = keys
                        .iter()
                        .zip(vals.iter())
                        .map(|(k, v)| {
                            let jv = match v {
                                Some(s) => serde_json::Value::String(s.clone()),
                                None    => serde_json::Value::Null,
                            };
                            (k.clone(), jv)
                        })
                        .collect();
                    entries.push(serde_json::Value::Object(obj));
                }
            }

            let json = serde_json::to_string_pretty(&entries)?;
            std::fs::write(&output, json)?;
            println!(
                "{}",
                format!(
                    "Wrote {} authorized entries to {} (keys: {}).",
                    entries.len(),
                    output.display(),
                    keys.join(", ")
                )
                .green()
            );
        }

        Commands::Violations { auth, keys, display: disp, source, filter } => {
            let auth_text = std::fs::read_to_string(&auth)
                .with_context(|| format!("Cannot read {}", auth.display()))?;
            let auth_entries: Vec<serde_json::Value> =
                serde_json::from_str(&auth_text)?;

            let (files, use_jctl) = resolve_source(&source);
            let events = read_and_parse(&files, use_jctl, source.quiet)?;
            let events = apply_filters(events, &FilterOpts::from(filter));

            let violations: Vec<UsbEvent> = events
                .into_iter()
                .filter(|e| {
                    !auth_entries.iter().any(|entry| {
                        keys.iter().all(|k| {
                            let ev = e.get_field(k);
                            let av = entry.get(k).and_then(|v| v.as_str());
                            ev == av
                        })
                    })
                })
                .collect();

            if violations.is_empty() {
                println!("{}", "No violations found.".green().bold());
            } else {
                println!(
                    "{}",
                    format!("{} violation(s) found!", violations.len())
                        .red()
                        .bold()
                );
                display(&violations, &disp, true);
            }
        }
    }

    Ok(())
}
