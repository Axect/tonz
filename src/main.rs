mod display;
mod scanner;
mod types;

use clap::Parser;
use crossterm::tty::IsTty;
use std::path::PathBuf;

use types::{Config, DisplayMode};

/// heft — See what's heavy
#[derive(Parser, Debug)]
#[command(
    name = "heft",
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
    #[arg(long)]
    json: bool,

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
}

fn main() {
    let cli = Cli::parse();

    let config = Config {
        path: cli.path,
        show_hidden: cli.show_hidden,
        json: cli.json,
        sparkline: cli.sparkline,
        across_mounts: cli.across_mounts,
        jobs: cli.jobs,
        no_color: cli.no_color,
    };

    let mode = if config.json {
        DisplayMode::Json
    } else if std::io::stdout().is_tty() {
        DisplayMode::Tty
    } else {
        DisplayMode::Pipe
    };

    let result = scanner::scan(&config);

    display::render(&result, &config, mode);
}
