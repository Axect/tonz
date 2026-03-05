# heft

**See what's heavy.** A modern, fast disk usage viewer for the terminal.

```
$ heft ~
  Documents                 ████████████████████░░░░   4.2 GiB  38%
  .cache                    ██████████░░░░░░░░░░░░░░   2.1 GiB  19%  (!)
  Downloads                 █████████░░░░░░░░░░░░░░░   1.8 GiB  16%
  Pictures                  ████░░░░░░░░░░░░░░░░░░░░   892 MiB   8%
  src                       ███░░░░░░░░░░░░░░░░░░░░░   614 MiB   6%
  ... 8 hidden items                                   312 MiB   3%
  Total: 11.2 GiB across 1,847 items
  [2 paths restricted]
```

Unlike `du` or `dust`, heft shows only what matters: the current directory's immediate children, sorted by size, with proportional bars. No deep trees, no clutter.

## Features

- **Fast** — Always-on parallel scanning via [Rayon](https://github.com/rayon-rs/rayon). 1.8x faster than dust on typical workloads
- **Readable** — Clean columnar output with proportional bars and 3-level semantic coloring
- **Smart hidden files** — Hidden items aggregated by default; large ones (>5% of total) auto-promoted with `(!)` marker
- **Unix-friendly** — TTY mode for humans, raw TSV for pipes, `--json` for scripts. Respects `NO_COLOR`
- **Safe** — Stays on one filesystem by default. Hardlink deduplication. Memory guard for huge directories
- **Small** — ~900 KiB stripped binary

## Install

### From source

```sh
cargo install --path .
```

### Build from git

```sh
git clone https://github.com/Axect/heft.git
cd heft
cargo build --release
# Binary at target/release/heft
```

## Usage

```
heft [PATH] [OPTIONS]
```

| Flag | Description |
|------|-------------|
| `-H`, `--hidden` | Show all hidden files and directories |
| `--json` | Output as line-delimited JSON |
| `--sparkline` | Use compact sparkline visualization |
| `-j`, `--jobs <N>` | Number of threads (default: auto) |
| `--across-mounts` | Cross filesystem boundaries |
| `--no-color` | Disable colors |

### Examples

```sh
# Current directory
heft

# Specific path
heft /var/log

# Show hidden files
heft -H ~

# Pipe-friendly (raw bytes + tab + name)
heft ~ | sort -rn | head -5

# JSON for scripting
heft --json ~/projects | jq '.name'
```

### Output Modes

**TTY** (default when connected to a terminal):
```
  Documents       ████████████████████░░░░   4.2 GiB  38%
  Downloads       █████████░░░░░░░░░░░░░░░   1.8 GiB  16%
```

**Pipe** (when piped to another command):
```
4509715456	Documents
1932735283	Downloads
```

**JSON** (`--json`):
```json
{"name":"Documents","size":4509715456,"is_dir":true,"is_hidden":false,"is_estimate":false,"percentage":38.2}
```

## How It Works

heft uses a **4-phase scanner pipeline**:

1. **Classify** — Read immediate children, check filesystem boundaries
2. **Parallel Scan** — Rayon work-stealing traversal per subdirectory with hardlink dedup
3. **Merge** — Aggregate sizes, errors, and estimate flags
4. **Render** — TTY/Pipe/JSON output with visualization

The depth-1-only constraint is both a UX and performance decision: no tree construction means minimal allocation and sub-second results even on large directories.

## License

MIT
