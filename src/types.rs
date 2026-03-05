use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct EntryInfo {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_estimate: bool,
}

#[derive(Debug)]
pub struct ScanResult {
    pub entries: Vec<EntryInfo>,
    pub hidden_entries: Vec<EntryInfo>,
    pub total_size: u64,
    pub error_count: u32,
    pub skipped_mounts: u32,
    pub restricted_count: u32,
    pub estimated_count: u32,
}

impl ScanResult {
    #[allow(dead_code)]
    pub fn hidden_total_size(&self) -> u64 {
        self.hidden_entries.iter().map(|e| e.size).sum()
    }

    #[allow(dead_code)]
    pub fn visible_total_size(&self) -> u64 {
        self.entries.iter().map(|e| e.size).sum()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Tty,
    Pipe,
    Json,
}

#[derive(Debug)]
pub struct Config {
    pub path: PathBuf,
    pub show_hidden: bool,
    pub json: bool,
    pub sparkline: bool,
    pub across_mounts: bool,
    pub jobs: Option<usize>,
    pub no_color: bool,
}
