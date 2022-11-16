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

## License

Licensed under MIT.
