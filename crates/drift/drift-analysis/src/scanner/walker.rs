//! Parallel file walker using the `ignore` crate's `WalkParallel`.
//!
//! Supports `.driftignore` (gitignore syntax, hierarchical) and 18 default ignore patterns.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crossbeam_channel as channel;
use drift_core::config::ScanConfig;

use super::language_detect::Language;
use super::types::DiscoveredFile;

/// The 18 default ignore patterns applied to every scan.
pub const DEFAULT_IGNORES: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    ".next",
    ".nuxt",
    "__pycache__",
    ".pytest_cache",
    "coverage",
    ".nyc_output",
    "vendor",
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    "bin",
    "obj",
];

/// Walk a directory tree in parallel, collecting discovered files.
///
/// Respects `.gitignore`, `.driftignore`, and the 18 default ignore patterns.
/// Returns files sorted by path for deterministic output.
pub fn walk_directory(
    root: &Path,
    config: &ScanConfig,
    cancelled: &AtomicBool,
) -> Result<Vec<DiscoveredFile>, drift_core::errors::ScanError> {
    let (tx, rx) = channel::unbounded();

    let max_file_size = config.effective_max_file_size();
    let follow_links = config.follow_symlinks.unwrap_or(false);
    let threads = config.effective_threads();

    let mut builder = ignore::WalkBuilder::new(root);
    builder
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .add_custom_ignore_filename(".driftignore")
        .max_filesize(Some(max_file_size))
        .follow_links(follow_links);

    if threads > 0 {
        builder.threads(threads);
    }

    // Add default ignore patterns via overrides
    let mut overrides = ignore::overrides::OverrideBuilder::new(root);
    for pattern in DEFAULT_IGNORES {
        // Negate pattern: !pattern/** means "ignore this directory"
        let _ = overrides.add(&format!("!{}/**", pattern));
        let _ = overrides.add(&format!("!{}", pattern));
    }
    // Add user-configured extra ignores
    for pattern in &config.extra_ignore {
        let _ = overrides.add(&format!("!{}", pattern));
    }
    if let Ok(built) = overrides.build() {
        builder.overrides(built);
    }

    let walker = builder.build_parallel();

    // Use Arc<AtomicBool> for safe cross-thread sharing
    let cancelled = Arc::new(AtomicBool::new(cancelled.load(Ordering::Relaxed)));

    walker.run(|| {
        let tx = tx.clone();
        let cancelled = Arc::clone(&cancelled);
        Box::new(move |entry| {
            if cancelled.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let entry = match entry {
                Ok(e) => e,
                Err(_) => return ignore::WalkState::Continue,
            };

            // Only process regular files
            let ft = match entry.file_type() {
                Some(ft) if ft.is_file() => ft,
                _ => return ignore::WalkState::Continue,
            };
            let _ = ft; // used above

            let path = entry.path().to_path_buf();
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(_) => return ignore::WalkState::Continue,
            };

            let language = Language::from_extension(
                path.extension().and_then(|e| e.to_str()),
            );

            let mtime = metadata
                .modified()
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

            let _ = tx.send(DiscoveredFile {
                path,
                file_size: metadata.len(),
                mtime,
                language,
            });

            ignore::WalkState::Continue
        })
    });

    drop(tx);
    let mut files: Vec<DiscoveredFile> = rx.into_iter().collect();
    // Sort for deterministic output
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}
