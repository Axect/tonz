use crate::types::{Config, DisplayMode, EntryInfo, ScanResult};
use owo_colors::OwoColorize;

const SIZE_COL_WIDTH: usize = 9;
const MIN_BAR_WIDTH: usize = 10;
const MAX_NAME_WIDTH: usize = 30;
const PROMOTE_THRESHOLD: f64 = 0.05;
const HIGHLIGHT_THRESHOLD: f64 = 0.25;
const MUTED_THRESHOLD: f64 = 0.02;

pub fn render(result: &ScanResult, config: &Config, mode: DisplayMode) {
    match mode {
        DisplayMode::Tty => render_tty(result, config),
        DisplayMode::Pipe => render_pipe(result, config),
        DisplayMode::Json => render_json(result, config),
    }
}

fn render_tty(result: &ScanResult, config: &Config) {
    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80)
        .max(40);
    let color = supports_color(config);

    let (mut display_entries, hidden_aggregate) = prepare_entries(result, config);
    display_entries.sort_by(|a, b| b.size.cmp(&a.size));

    let total = result.total_size.max(1);

    // Column widths
    // Overhead: indent(2) + gap(2) + gap(2) + SIZE_COL_WIDTH(9) + gap(2) + pct(4) = 21
    // Add 5 for "  (!)" marker when promoted hidden entries exist
    let has_promoted_hidden = !config.show_hidden
        && result.hidden_entries.iter().any(|e| {
            (e.size as f64 / total as f64) > PROMOTE_THRESHOLD
        });
    let overhead = 2 + 2 + 2 + SIZE_COL_WIDTH + 2 + 4 + if has_promoted_hidden { 5 } else { 0 };
    let mut name_width = MAX_NAME_WIDTH.min(term_width / 3);
    let mut remaining = term_width.saturating_sub(name_width + overhead);
    if !config.sparkline && remaining < MIN_BAR_WIDTH {
        let deficit = MIN_BAR_WIDTH - remaining;
        name_width = name_width.saturating_sub(deficit);
        remaining = term_width.saturating_sub(name_width + overhead);
    }
    let bar_width = if config.sparkline { 3 } else { remaining };

    for entry in &display_entries {
        let ratio = entry.size as f64 / total as f64;
        let pct = (ratio * 100.0) as u64;
        let display_name = if entry.is_symlink {
            format!("@{}", entry.name)
        } else {
            entry.name.clone()
        };
        let name_display = truncate_name(&display_name, name_width);
        let bar = if config.sparkline {
            make_sparkline(ratio)
        } else {
            make_bar(ratio, bar_width)
        };
        let (raw_size, size_unit) = format_size(entry.size);
        let size_num = if entry.is_estimate {
            format!("~{}", raw_size)
        } else {
            raw_size
        };

        let marker = if entry.is_hidden { "  (!)" } else { "" };

        // Pad plain strings first, then apply color styling
        // (ANSI escape codes break Rust's format width calculation)
        let padded_name = format!("{:<width$}", name_display, width = name_width);
        let size_width = SIZE_COL_WIDTH.saturating_sub(size_unit.len() + 1);
        let padded_size = format!("{:>width$}", size_num, width = size_width);
        let padded_pct = format!("{:>3}%", pct);

        if color {
            if entry.is_symlink {
                print!("  {}", padded_name.magenta());
                print!("  {}", bar.dimmed());
                print!("  {}", padded_size.dimmed());
                print!(" {}", size_unit.dimmed());
                print!("  {}", padded_pct.dimmed());
            } else if ratio > HIGHLIGHT_THRESHOLD {
                print!("  {}", padded_name.bold().bright_cyan());
                print!("  {}", bar.bold());
                print!("  {}", padded_size.bold());
                print!(" {}", size_unit.dimmed());
                print!("  {}", padded_pct);
                if !marker.is_empty() {
                    print!("{}", marker.yellow());
                }
            } else if ratio < MUTED_THRESHOLD {
                print!("  {}", padded_name.dimmed());
                print!("  {}", bar.dimmed());
                print!("  {}", padded_size.dimmed());
                print!(" {}", size_unit.dimmed());
                print!("  {}", padded_pct.dimmed());
                if !marker.is_empty() {
                    print!("{}", marker.dimmed());
                }
            } else {
                print!("  {}", padded_name);
                print!("  {}", bar);
                print!("  {}", padded_size);
                print!(" {}", size_unit.dimmed());
                print!("  {}", padded_pct);
                if !marker.is_empty() {
                    print!("{}", marker.yellow());
                }
            }
        } else {
            print!("  {}", padded_name);
            print!("  {}", bar);
            print!("  {}", padded_size);
            print!(" {}", size_unit);
            print!("  {}{}", padded_pct, marker);
        }
        println!();
    }

    // Hidden aggregate line
    if let Some((count, agg_size)) = hidden_aggregate {
        let ratio = agg_size as f64 / total as f64;
        let pct = (ratio * 100.0) as u64;
        let label = format!("... {} hidden items", count);
        let (size_num, size_unit) = format_size(agg_size);
        let bar_placeholder = " ".repeat(bar_width);
        let padded_label = format!("{:<width$}", label, width = name_width);
        let size_width = SIZE_COL_WIDTH.saturating_sub(size_unit.len() + 1);
        let padded_size = format!("{:>width$}", size_num, width = size_width);
        let padded_pct = format!("{:>3}%", pct);

        if color {
            print!("  {}", padded_label.dimmed());
            print!("  {}", bar_placeholder);
            print!("  {}", padded_size.dimmed());
            print!(" {}", size_unit.dimmed());
            print!("  {}", padded_pct.dimmed());
        } else {
            print!("  {}", padded_label);
            print!("  {}", bar_placeholder);
            print!("  {}", padded_size);
            print!(" {}", size_unit);
            print!("  {}", padded_pct);
        }
        println!();
    }

    // Footer
    let item_count = result.entries.len() + result.hidden_entries.len();
    let (total_num, total_unit) = format_size(result.total_size);
    println!(
        "  Total: {} {} across {} items",
        total_num, total_unit, item_count
    );

    let mut notes = Vec::new();
    if result.error_count > 0 {
        notes.push(format!("{} errors", result.error_count));
    }
    if result.restricted_count > 0 {
        notes.push(format!("{} paths restricted", result.restricted_count));
    }
    if result.skipped_mounts > 0 {
        notes.push(format!("{} mount points skipped", result.skipped_mounts));
    }
    if result.estimated_count > 0 {
        notes.push(format!(
            "{} entries estimated (too many files)",
            result.estimated_count
        ));
    }
    if !notes.is_empty() {
        println!("  [{}]", notes.join(", "));
    }
}

fn render_pipe(result: &ScanResult, config: &Config) {
    let mut all: Vec<&EntryInfo> = result.entries.iter().collect();
    if config.show_hidden {
        all.extend(result.hidden_entries.iter());
    }
    all.sort_by(|a, b| b.size.cmp(&a.size));

    for entry in &all {
        let prefix = if entry.is_estimate { "~" } else { "" };
        println!("{}{}\t{}", prefix, entry.size, entry.name);
    }
}

fn render_json(result: &ScanResult, config: &Config) {
    let total = result.total_size.max(1) as f64;
    let mut all: Vec<&EntryInfo> = result.entries.iter().collect();
    if config.show_hidden {
        all.extend(result.hidden_entries.iter());
    }
    all.sort_by(|a, b| b.size.cmp(&a.size));

    for entry in &all {
        let pct = (entry.size as f64 / total) * 100.0;
        let name = entry.name.replace('\\', "\\\\").replace('"', "\\\"");
        println!(
            "{{\"name\":\"{}\",\"size\":{},\"is_dir\":{},\"is_hidden\":{},\"is_estimate\":{},\"is_symlink\":{},\"percentage\":{:.1}}}",
            name, entry.size, entry.is_dir, entry.is_hidden, entry.is_estimate, entry.is_symlink, pct
        );
    }
}

/// Prepare entries for TTY display. Returns (display_entries, optional hidden aggregate).
/// Hidden aggregate is (count, total_size) of non-promoted hidden entries.
fn prepare_entries(
    result: &ScanResult,
    config: &Config,
) -> (Vec<DisplayEntry>, Option<(usize, u64)>) {
    let total = result.total_size.max(1);
    let mut entries: Vec<DisplayEntry> = result
        .entries
        .iter()
        .map(|e| DisplayEntry {
            name: e.name.clone(),
            size: e.size,
            is_hidden: false,
            is_estimate: e.is_estimate,
            is_symlink: e.is_symlink,
        })
        .collect();

    if config.show_hidden {
        for e in &result.hidden_entries {
            entries.push(DisplayEntry {
                name: e.name.clone(),
                size: e.size,
                is_hidden: false,
                is_estimate: e.is_estimate,
                is_symlink: e.is_symlink,
            });
        }
        (entries, None)
    } else {
        let mut agg_size: u64 = 0;
        let mut agg_count: usize = 0;

        for e in &result.hidden_entries {
            let ratio = e.size as f64 / total as f64;
            if ratio > PROMOTE_THRESHOLD {
                entries.push(DisplayEntry {
                    name: e.name.clone(),
                    size: e.size,
                    is_hidden: true,
                    is_estimate: e.is_estimate,
                    is_symlink: e.is_symlink,
                });
            } else {
                agg_size += e.size;
                agg_count += 1;
            }
        }

        let aggregate = if agg_count > 0 {
            Some((agg_count, agg_size))
        } else {
            None
        };
        (entries, aggregate)
    }
}

struct DisplayEntry {
    name: String,
    size: u64,
    is_hidden: bool,
    is_estimate: bool,
    is_symlink: bool,
}

fn format_size(bytes: u64) -> (String, String) {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * KIB;
    const GIB: u64 = 1024 * MIB;
    const TIB: u64 = 1024 * GIB;

    if bytes >= TIB {
        (format!("{:.1}", bytes as f64 / TIB as f64), "TiB".into())
    } else if bytes >= GIB {
        (format!("{:.1}", bytes as f64 / GIB as f64), "GiB".into())
    } else if bytes >= MIB {
        (format!("{:.1}", bytes as f64 / MIB as f64), "MiB".into())
    } else if bytes >= KIB {
        (format!("{}", bytes / KIB), "KiB".into())
    } else {
        (format!("{}", bytes), "B".into())
    }
}

fn make_bar(ratio: f64, width: usize) -> String {
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let empty = width - filled;
    let mut bar = String::with_capacity(width * 3);
    for _ in 0..filled {
        bar.push('\u{2588}'); // █
    }
    for _ in 0..empty {
        bar.push('\u{2591}'); // ░
    }
    bar
}

fn make_sparkline(ratio: f64) -> String {
    const SPARKS: [char; 8] = [
        '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}',
        '\u{2588}',
    ];
    let idx = ((ratio * 7.0).round() as usize).min(7);
    let ch = SPARKS[idx];
    format!("{}{}{}", ch, ch, ch)
}

fn truncate_name(name: &str, max_width: usize) -> String {
    if max_width < 7 {
        // Too narrow for mid-ellipsis to make sense
        return name.chars().take(max_width).collect();
    }
    let char_count = name.chars().count();
    if char_count <= max_width {
        name.to_string()
    } else {
        let keep = max_width - 3; // 3 for "..."
        let front = keep / 2;
        let back = keep - front;
        let front_str: String = name.chars().take(front).collect();
        let back_str: String = name
            .chars()
            .rev()
            .take(back)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{}...{}", front_str, back_str)
    }
}

fn supports_color(config: &Config) -> bool {
    if config.no_color {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    use crossterm::tty::IsTty;
    std::io::stdout().is_tty()
}
