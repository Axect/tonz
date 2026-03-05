mod display;
mod scanner;
mod types;

use clap::Parser;
use crossterm::tty::IsTty;
use std::path::PathBuf;

use types::{Config, DisplayMode};

/// tonz — See what's heavy
#[derive(Parser, Debug)]
#[command(
    name = "tonz",
    version,
    about = "See what's heavy — a modern, fast disk usage viewer"
)]
struct Cli {
    /// Target directory to analyze
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Show hidden files and directories
    #[arg(short = 'H', long = "hidden")]
    show_hidden: bool,

    /// Output as line-delimited JSON
    #[arg(long, conflicts_with = "llm")]
    json: bool,

    /// Output in token-efficient format for AI agents
    #[arg(long, conflicts_with = "json")]
    llm: bool,

    /// Use compact sparkline visualization
    #[arg(long)]
    sparkline: bool,

    /// Number of threads (default: auto)
    #[arg(short = 'j', long = "jobs")]
    jobs: Option<usize>,

    /// Cross filesystem boundaries
    #[arg(long)]
    across_mounts: bool,

    /// Disable colors
    #[arg(long)]
    no_color: bool,

    /// Hide entries below this percentage of total size
    #[arg(long, value_name = "N")]
    threshold_pct: Option<f64>,

    /// Show only the top N entries by size
    #[arg(long, value_name = "N")]
    top: Option<usize>,
}

fn main() {
    let cli = Cli::parse();

    let config = Config {
        path: cli.path,
        show_hidden: cli.show_hidden,
        json: cli.json,
        llm: cli.llm,
        sparkline: cli.sparkline,
        across_mounts: cli.across_mounts,
        jobs: cli.jobs,
        no_color: cli.no_color,
        threshold_pct: cli.threshold_pct,
        top: cli.top,
    };

    let mode = if config.json {
        DisplayMode::Json
    } else if config.llm {
        DisplayMode::Llm
    } else if std::io::stdout().is_tty() {
        DisplayMode::Tty
    } else {
        DisplayMode::Pipe
    };

    let result = scanner::scan(&config);

    display::render(&result, &config, mode);

    // Semantic exit codes
    if result.error_count > 0 || result.restricted_count > 0 {
        std::process::exit(1);
    }
}
