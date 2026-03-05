use std::collections::HashSet;
use std::fs;
use std::path::Path;

use rayon::prelude::*;
use walkdir::WalkDir;

use crate::types::{Config, EntryInfo, ScanResult};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

const MAX_FILES_PER_SUBTREE: u64 = 10_000_000;

pub fn scan(config: &Config) -> ScanResult {
    let mut error_count: u32 = 0;
    let mut skipped_mounts: u32 = 0;
    let mut restricted_count: u32 = 0;

    #[cfg(unix)]
    let root_dev = if !config.across_mounts {
        fs::metadata(&config.path).ok().map(|m| m.dev())
    } else {
        None
    };
    #[cfg(not(unix))]
    let root_dev: Option<u64> = None;

    let dir_entries: Vec<_> = match fs::read_dir(&config.path) {
        Ok(rd) => rd
            .filter_map(|res| match res {
                Ok(e) => Some(e),
                Err(_) => {
                    error_count += 1;
                    None
                }
            })
            .collect(),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                restricted_count += 1;
            } else {
                error_count += 1;
            }
            return ScanResult {
                entries: Vec::new(),
                hidden_entries: Vec::new(),
                total_size: 0,
                error_count,
                skipped_mounts,
                restricted_count,
                estimated_count: 0,
            };
        }
    };

    // Phase 1: classify depth-1 children
    struct Child {
        name: String,
        path: std::path::PathBuf,
        is_dir: bool,
        is_hidden: bool,
        size: u64,
        needs_scan: bool,
        is_symlink: bool,
    }

    let mut children: Vec<Child> = Vec::with_capacity(dir_entries.len());

    for de in &dir_entries {
        let name = de.file_name().to_string_lossy().into_owned();
        let is_hidden = name.starts_with('.');
        let path = de.path();

        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::PermissionDenied {
                    restricted_count += 1;
                } else {
                    error_count += 1;
                }
                continue;
            }
        };

        if meta.is_dir() {
            #[cfg(unix)]
            #[allow(clippy::collapsible_if)]
            {
                if let Some(rd) = root_dev {
                    if meta.dev() != rd {
                        skipped_mounts += 1;
                        children.push(Child {
                            name,
                            path,
                            is_dir: true,
                            is_hidden,
                            size: 0,
                            needs_scan: false,
                            is_symlink: false,
                        });
                        continue;
                    }
                }
            }
            children.push(Child {
                name,
                path,
                is_dir: true,
                is_hidden,
                size: 0,
                needs_scan: true,
                is_symlink: false,
            });
        } else if meta.is_file() || meta.is_symlink() {
            children.push(Child {
                name,
                path: path.clone(),
                is_dir: false,
                is_hidden,
                size: meta.len(),
                needs_scan: false,
                is_symlink: meta.is_symlink(),
            });
        }
    }

    // Phase 2: scan directories with rayon (always parallel)
    let pool = config
        .jobs
        .and_then(|n| rayon::ThreadPoolBuilder::new().num_threads(n).build().ok());

    let scan_indices: Vec<usize> = children
        .iter()
        .enumerate()
        .filter(|(_, c)| c.needs_scan)
        .map(|(i, _)| i)
        .collect();

    struct ScanOut {
        index: usize,
        size: u64,
        errors: u32,
        skipped: u32,
        restricted: u32,
        is_estimate: bool,
    }

    let do_scan = || -> Vec<ScanOut> {
        scan_indices
            .par_iter()
            .map(|&idx| {
                let child = &children[idx];
                let mut seen = HashSet::new();
                let (size, errs, skips, restr, is_est) =
                    compute_dir_size(&child.path, root_dev, &mut seen);
                ScanOut {
                    index: idx,
                    size,
                    errors: errs,
                    skipped: skips,
                    restricted: restr,
                    is_estimate: is_est,
                }
            })
            .collect()
    };

    let scan_results = match pool {
        Some(ref p) => p.install(do_scan),
        None => do_scan(),
    };

    // Phase 3: merge results
    let mut estimate_set = HashSet::new();
    let mut estimated_count: u32 = 0;

    for r in &scan_results {
        children[r.index].size = r.size;
        error_count += r.errors;
        skipped_mounts += r.skipped;
        restricted_count += r.restricted;
        if r.is_estimate {
            estimate_set.insert(r.index);
            estimated_count += 1;
        }
    }

    // Phase 4: build output
    let mut entries = Vec::new();
    let mut hidden_entries = Vec::new();

    for (i, child) in children.into_iter().enumerate() {
        let info = EntryInfo {
            name: child.name,
            size: child.size,
            is_dir: child.is_dir,
            is_hidden: child.is_hidden,
            is_estimate: estimate_set.contains(&i),
            is_symlink: child.is_symlink,
        };
        if child.is_hidden {
            hidden_entries.push(info);
        } else {
            entries.push(info);
        }
    }

    let total_size: u64 = entries.iter().map(|e| e.size).sum::<u64>()
        + hidden_entries.iter().map(|e| e.size).sum::<u64>();

    ScanResult {
        entries,
        hidden_entries,
        total_size,
        error_count,
        skipped_mounts,
        restricted_count,
        estimated_count,
    }
}

fn compute_dir_size(
    path: &Path,
    root_dev: Option<u64>,
    seen_inodes: &mut HashSet<(u64, u64)>,
) -> (u64, u32, u32, u32, bool) {
    let mut size: u64 = 0;
    let mut errors: u32 = 0;
    let mut skipped_mounts: u32 = 0;
    let mut restricted: u32 = 0;
    let mut file_count: u64 = 0;

    for result in WalkDir::new(path) {
        let entry = match result {
            Ok(e) => e,
            Err(err) => {
                if err
                    .io_error()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
                {
                    restricted += 1;
                } else {
                    errors += 1;
                }
                continue;
            }
        };

        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                if e.io_error()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::PermissionDenied)
                {
                    restricted += 1;
                } else {
                    errors += 1;
                }
                continue;
            }
        };

        if meta.is_dir() {
            continue;
        }

        #[cfg(unix)]
        #[allow(clippy::collapsible_if)]
        {
            if let Some(rd) = root_dev {
                if meta.dev() != rd {
                    skipped_mounts += 1;
                    continue;
                }
            }

            let key = (meta.dev(), meta.ino());
            if meta.nlink() > 1 && !seen_inodes.insert(key) {
                continue;
            }
        }

        size += meta.len();
        file_count += 1;

        if file_count >= MAX_FILES_PER_SUBTREE {
            return (size, errors, skipped_mounts, restricted, true);
        }
    }

    (size, errors, skipped_mounts, restricted, false)
}
