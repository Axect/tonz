use std::process::Command;

fn tonz() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tonz"))
}

fn fixture_path() -> String {
    format!("{}/tests/fixtures/sample_dir", env!("CARGO_MANIFEST_DIR"))
}

// ── --llm mode ──────────────────────────────────────────────

#[test]
fn llm_mode_has_summary_header() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next().unwrap();
    // Header format: {path} ({size}, {N} items)
    assert!(
        first_line.contains("items)"),
        "header missing 'items)': {first_line}"
    );
    assert!(
        first_line.contains("M"),
        "header should have human-readable size"
    );
}

#[test]
fn llm_mode_tab_delimited() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Skip header, check data lines are tab-delimited
    for line in stdout.lines().skip(1) {
        let tabs = line.matches('\t').count();
        assert_eq!(tabs, 2, "expected 2 tabs per data line, got {tabs}: {line}");
    }
}

#[test]
fn llm_mode_dirs_have_slash_suffix() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // subdir_a is a directory and should end with /
    let subdir_line = stdout
        .lines()
        .find(|l| l.contains("subdir_a"))
        .expect("subdir_a not found");
    assert!(
        subdir_line.ends_with('/'),
        "dir should end with /: {subdir_line}"
    );
}

#[test]
fn llm_mode_files_no_slash() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let file_line = stdout
        .lines()
        .find(|l| l.contains("small.txt"))
        .expect("small.txt not found");
    assert!(
        !file_line.ends_with('/'),
        "file should not end with /: {file_line}"
    );
}

#[test]
fn llm_mode_human_readable_sizes() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // subdir_a has a 10M file → should show "10M" or "10.0M"
    let line = stdout
        .lines()
        .find(|l| l.contains("subdir_a"))
        .expect("subdir_a not found");
    assert!(line.contains("M"), "10MB dir should show M unit: {line}");
}

// ── --threshold-pct ─────────────────────────────────────────

#[test]
fn threshold_pct_filters_small_entries() {
    let output = tonz()
        .args(["--llm", "--threshold-pct", "5", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // subdir_a (~10M) is ~95% of total → should appear
    assert!(
        stdout.contains("subdir_a"),
        "large dir should pass threshold"
    );
    // small.txt (100B) is ~0% → should be filtered
    assert!(
        !stdout.contains("small.txt"),
        "tiny file should be filtered"
    );
}

#[test]
fn threshold_pct_shows_filtered_count_in_header() {
    let output = tonz()
        .args(["--llm", "--threshold-pct", "5", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let header = stdout.lines().next().unwrap();
    assert!(
        header.contains("shown"),
        "header should show filtered count: {header}"
    );
}

#[test]
fn threshold_pct_100_shows_diagnostic() {
    let output = tonz()
        .args(["--llm", "--threshold-pct", "100", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("below 100% threshold"),
        "should show diagnostic when all filtered: {stdout}"
    );
}

// ── --top ───────────────────────────────────────────────────

#[test]
fn top_limits_entries() {
    let output = tonz()
        .args(["--llm", "--top", "2", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let data_lines: Vec<&str> = stdout.lines().skip(1).collect();
    assert_eq!(data_lines.len(), 2, "--top 2 should show exactly 2 entries");
}

#[test]
fn top_and_threshold_union() {
    // --threshold-pct 90 would show only subdir_a, but --top 2 adds the next one
    let output = tonz()
        .args([
            "--llm",
            "--threshold-pct",
            "90",
            "--top",
            "2",
            &fixture_path(),
        ])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let data_lines: Vec<&str> = stdout.lines().skip(1).collect();
    assert_eq!(
        data_lines.len(),
        2,
        "union of threshold + top should show 2 entries"
    );
}

// ── --json + threshold ──────────────────────────────────────

#[test]
fn json_threshold_filters() {
    let output = tonz()
        .args(["--json", "--threshold-pct", "5", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    // Only subdir_a should pass 5% threshold
    assert_eq!(lines.len(), 1, "JSON with 5% threshold should show 1 entry");
    assert!(lines[0].contains("subdir_a"), "should be the large dir");
}

// ── --llm + --json conflict ─────────────────────────────────

#[test]
fn llm_json_conflict() {
    let output = tonz()
        .args(["--llm", "--json", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    assert_eq!(
        output.status.code(),
        Some(2),
        "conflicting flags should exit 2"
    );
}

// ── Exit codes ──────────────────────────────────────────────

#[test]
fn exit_code_0_on_clean_scan() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    assert_eq!(output.status.code(), Some(0), "clean scan should exit 0");
}

// ── Filenames with spaces ───────────────────────────────────

#[test]
fn llm_handles_spaces_in_names() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|l| l.contains("dir with spaces"))
        .expect("dir with spaces not found");
    // Tab-delimited: size\tpct%\tname/
    let parts: Vec<&str> = line.split('\t').collect();
    assert_eq!(parts.len(), 3, "should have 3 tab-separated fields");
    assert!(
        parts[2].starts_with("dir with spaces"),
        "name field should contain full name with spaces"
    );
}

// ── format_size_human (tested via output) ───────────────────

#[test]
fn format_sizes_are_compact() {
    let output = tonz()
        .args(["--llm", &fixture_path()])
        .output()
        .expect("failed to run tonz");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let size_field = line.split('\t').next().unwrap();
        // Human-readable sizes should be short: "10M", "500K", "100B", "200K"
        assert!(
            size_field.len() <= 6,
            "size should be compact (<=6 chars): '{size_field}'"
        );
    }
}
