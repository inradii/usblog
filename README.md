# usblog

`usblog` collects USB events from system logs and builds a timeline of connections.
It is inspired by [usbrip](https://github.com/snovvcrash/usbrip), which was unfortunately archived.

## Installation

```bash
cargo install --path .
```

## Usage

```sh
usblog [SUBCOMMAND]
```

Running `usblog` without a subcommand is equivalent to `usblog history`.

### Subcommands

| Subcommand   | Description                                           |
| ------------ | ----------------------------------------------------- |
| `history`    | Retrieve USB connection history from system logs      |
| `generate`   | Generate an authorized device list from current state |
| `live`       | Show USB events live                                  |
| `violations` | Find events absent from an authorized device list     |

### history

```sh
usblog history [OPTIONS]
```

| Option              | Description                                                            |
| ------------------- | ---------------------------------------------------------------------- |
| `-f`, `--file`      | Log files to parse (default: `/var/log/syslog*`, `/var/log/messages*`) |
| `-j`, `--journalctl`| Read from journalctl instead of log files                              |
| `-q`, `--quiet`     | Suppress progress messages                                             |
| `-o`, `--output`    | Save results to a JSON file                                            |
| `-t`, `--table`     | Display as a table (default)                                           |
| `-l`, `--list`      | Display as a vertical list                                             |

Filter options (all repeatable):

| Option        | Description                        |
| ------------- | ---------------------------------- |
| `-d`, `--date`| Filter by date prefix (YYYY-MM-DD) |
| `--host`      | Filter by hostname                 |
| `--vid`       | Filter by vendor ID                |
| `--pid`       | Filter by product ID               |
| `--prod`      | Filter by product name (substring) |
| `--manufact`  | Filter by manufacturer (substring) |
| `--serial`    | Filter by serial number            |
| `--port`      | Filter by port                     |
| `-n`          | Limit to N results                 |

### generate

Snapshot the currently seen devices into an authorization list (JSON).

```sh
usblog generate <OUTPUT> [OPTIONS]
```

| Option        | Description                                                                                             |
| ------------- | ------------------------------------------------------------------------------------------------------- |
| `<OUTPUT>`    | Path to write the authorization JSON file                                                               |
| `--key`       | Fields used as keys: `vid`, `pid`, `prod`, `manufact`, `serial`, `port`, `host` (default: `vid`, `pid`) |

Accepts the same source and filter options as the subcommand `history`.

### violations

Report events not present in an authorization list.

```sh
usblog violations <AUTH> [OPTIONS]
```

| Option        | Description                                                              |
| ------------- | ------------------------------------------------------------------------ |
| `<AUTH>`      | Path to the authorization JSON file                                      |
| `--key`       | Fields to match against the authorization list (default: `vid`, `pid`)   |

Accepts the same source, display and filter options as the subcommand `history`.

## Use case - Detect a rogue device (e.g., USB Rubber Ducky)

A USB Rubber Ducky enumerates as a keyboard but is manufactured by Hak5.
The quickest check is to search the full history for its vendor or product strings:

```sh
# Search by manufacturer name
usblog history --manufact "Hak5"

# Search by product name
usblog history --prod "Rubber Ducky"

# Search by known vendor ID (Hak5 VID: 0xf1f1)
usblog history --vid f1f1
```

For systematic detection, generate an authorization list from a known-good
state and then flag anything that falls outside it:

```sh
# 1. Snapshot the currently trusted devices
usblog generate authorized.json --key vid,pid,serial

# 2. At any later point, report devices not in the list
usblog violations authorized.json --key vid,pid,serial
```

Any device not in `authorized.json`, including a Rubber Ducky plugged in
after the snapshot, will be reported as a violation.

## License

Licensed under MIT.
