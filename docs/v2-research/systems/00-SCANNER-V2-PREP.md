# Scanner v2 — Implementation Preparation

> Synthesized from: 00-SCANNER.md, DRIFT-V2-FULL-SYSTEM-AUDIT.md, DRIFT-V2-STACK-HIERARCHY.md,
> PLANNING-DRIFT.md, 01-PARSERS.md, 02-STORAGE.md, 03-NAPI-BRIDGE.md, 04-INFRASTRUCTURE.md,
> 05-CALL-GRAPH.md, older research (01-rust-core/scanner.md, 25-services-layer/scanner-service.md),
> and internet validation of crate choices.
>
> Purpose: Everything needed to build the scanner. Decisions resolved, inconsistencies flagged,
> interface contracts defined, build order specified.

---

## 1. Role in the System

The scanner is Level 0 bedrock. It's the entry point to the entire Drift pipeline — every analysis path starts with "which files exist and which changed?" Nothing runs without it.

Downstream consumers (all depend on scanner output):
- Parsers (receive file list + content)
- Incremental detection (consume content hashes + mtime)
- Call graph builder (needs file list for parallel extraction)
- Every detector, analyzer, and enforcement system (transitively)

Upstream dependencies (must exist before scanner):
- Configuration system (ignore patterns, max file size, thread count)
- Error handling infrastructure (ScanError enum via thiserror)
- Tracing infrastructure (scan spans, file-level spans)
- DriftEventHandler trait (on_scan_started, on_scan_progress, on_scan_complete)

---

## 2. Resolved Inconsistency: `ignore` Crate vs `walkdir` + `rayon`

The older research docs (01-rust-core/scanner.md, MASTER_RECAP.md, directory maps, systems reference)
all reference `walkdir` + `rayon` as the file walking approach. The newer 00-SCANNER.md research doc
explicitly evaluates this and recommends the `ignore` crate instead.

**Resolution: Use the `ignore` crate.** The scanner research doc's analysis is correct:

- The `ignore` crate IS the parallel walker extracted from ripgrep by BurntSushi
- It subsumes `walkdir` (same author) and adds: parallel walking via `WalkParallel`,
  native `.gitignore` support (nested, hierarchical), custom ignore file support,
  `.git/info/exclude` and global gitignore respect, file type filtering
- Battle-tested in ripgrep, fd, delta, difftastic (80M+ downloads on crates.io as of 2025)
- Using `walkdir` + manual gitignore + rayon is strictly worse — it's reinventing what
  `ignore` already provides

The `ignore` crate's `WalkParallel` uses an internal work-stealing thread pool.
`rayon` is still needed for the post-discovery processing phase (hashing, metadata collection).

**Cargo.toml dependencies for the scanner:**
```toml
[dependencies]
ignore = "0.4"           # Parallel file walking + gitignore
rayon = "1.10"           # Parallel processing of discovered files
xxhash-rust = { version = "0.8", features = ["xxh3"] }  # Content hashing
num_cpus = "1.16"        # Thread count detection
```

---

## 3. Two-Phase Architecture (Final Decision)

The scanner doc evaluates two approaches and lands on a hybrid:

**Phase 1 — Discovery** (`ignore::WalkParallel`)
- Walk filesystem in parallel
- Collect paths into a `Vec<PathBuf>` (needed for total count → progress reporting)
- Apply ignore rules (.gitignore, .driftignore, max file size)
- This gives us the total file count upfront for progress callbacks

**Phase 2 — Processing** (`rayon::par_iter`)
- Hash file contents (xxh3)
- Collect file metadata (mtime, size)
- Compare against cached state in drift.db `file_metadata` table
- Classify each file: added / modified / removed / unchanged

Why two phases instead of doing everything in `WalkParallel` callbacks:
- Progress reporting requires knowing total count upfront (audit requires AtomicU64 counter
  + ThreadsafeFunction every 100 files)
- Rayon's work-stealing is better suited for the CPU-bound hashing work
- Clean separation of I/O-bound discovery from CPU-bound processing

---

## 4. Content Hashing: xxh3 (Confirmed)

**Primary: xxh3 via `xxhash-rust`** (latest: 0.8.x as of late 2025)

Validation from benchmarks ([rosetta-hashing](https://blog.goose.love/posts/rosetta-hashing/)):
- xxh3: ~580µs for large inputs, portable, deterministic, excellent collision resistance
- For typical source files (10-50KB): microseconds per file
- 100K files at 20KB average: ~200ms total hashing time

**Optional: blake3 behind config flag**
- ~8x slower than xxh3 but still fast in absolute terms
- Cryptographic quality — useful for audit trails, lock file verification (enterprise)
- Only matters at scale: 100K files saves ~400ms with xxh3 over blake3
- Add as `DriftConfig.scan.hash_algorithm: "xxh3" | "blake3"` if enterprise features need it

**Not suitable:**
- ahash: output not stable across versions/platforms (uses random state). Cannot be persisted.
- SipHash (std): 4x slower than xxh3, no benefit for this use case
- FNV: 15x slower, no benefit

---

## 5. Incremental Detection: Two-Level Strategy

This is the core value of the scanner — avoiding redundant work.

### Level 1: mtime comparison (instant, catches ~90% of unchanged files)
```
if file.mtime == cached.mtime → skip (unchanged)
```

### Level 2: content hash (for mtime-changed files)
```
if file.mtime != cached.mtime → compute xxh3 hash
  if hash == cached.hash → unchanged (update mtime in cache, skip re-analysis)
  if hash != cached.hash → modified (needs re-analysis)
```

This handles git operations, `touch`, editor save-without-change, etc.

### Storage Schema (in drift.db)
```sql
CREATE TABLE file_metadata (
    path TEXT PRIMARY KEY,
    content_hash BLOB NOT NULL,    -- xxh3 hash (8 bytes)
    mtime_secs INTEGER NOT NULL,
    mtime_nanos INTEGER NOT NULL,
    file_size INTEGER NOT NULL,
    last_indexed_at INTEGER NOT NULL
) STRICT;

CREATE INDEX idx_file_metadata_hash ON file_metadata(content_hash);
```

### The diff() Output
```rust
pub struct ScanDiff {
    pub added: Vec<PathBuf>,      // new files not in cache
    pub modified: Vec<PathBuf>,   // content hash changed
    pub removed: Vec<PathBuf>,    // in cache but not on disk
    pub unchanged: Vec<PathBuf>,  // same content hash (or same mtime)
}
```

### Algorithm
1. Walk filesystem → collect current files with mtime
2. Load `file_metadata` from drift.db
3. For each current file:
   - Not in cache → `added`
   - In cache, mtime unchanged → `unchanged` (skip hash)
   - In cache, mtime changed → compute hash →
     - hash differs: `modified`
     - hash same: `unchanged` (update mtime in cache)
4. For each cached file not in current set → `removed`

Same strategy as git's index and rust-analyzer's VFS.

---

## 6. Three-Layer Incrementality (from AD1)

The scanner owns Layer 1. Layers 2 and 3 are downstream but the scanner's output drives them:

- **Layer 1** (scanner): File-level skip via content hash — scanner's core job
- **Layer 2** (detectors): Pattern re-scoring only for changed files — consumers of ScanDiff
- **Layer 3** (conventions): Re-learning threshold — if >10% files changed, trigger full re-learn

The scanner must provide enough information for downstream systems to make Layer 2/3 decisions.
This means `ScanDiff` needs to include the ratio: `modified.len() / (total files)`.

---

## 7. Configuration

From the infrastructure doc (04-INFRASTRUCTURE.md), config is TOML-based:

```toml
[scan]
max_file_size = 1_048_576  # 1MB default
threads = 0                 # 0 = auto-detect (num_cpus)
extra_ignore = ["*.generated.ts", "vendor/"]
follow_symlinks = false
compute_hashes = true       # can disable for fast file-list-only mode
force_full_scan = false     # skip mtime optimization, re-hash everything
skip_binary = true          # skip binary files by default
```

### Rust Config Struct
```rust
#[derive(Deserialize, Default)]
pub struct ScanConfig {
    /// Maximum file size in bytes. Files larger than this are skipped.
    /// Default: 1MB (1_048_576). Almost all source files are under this.
    /// Files over 1MB are typically generated code, bundles, or data files.
    pub max_file_size: Option<u64>,

    /// Number of threads for parallel processing. 0 = auto-detect.
    pub threads: Option<usize>,

    /// Additional ignore patterns beyond .gitignore/.driftignore.
    pub extra_ignore: Vec<String>,

    /// Whether to follow symbolic links. Default: false.
    pub follow_symlinks: Option<bool>,

    /// Whether to compute content hashes. Default: true.
    /// Set to false for fast file-list-only mode.
    pub compute_hashes: Option<bool>,

    /// Force full rescan, skipping mtime optimization. Default: false.
    /// Useful after git operations that touch many file mtimes.
    pub force_full_scan: Option<bool>,

    /// Skip binary files (detected via null-byte heuristic). Default: true.
    pub skip_binary: Option<bool>,
}
```

---

## 8. .driftignore Format

Gitignore syntax exactly. No new format to learn.

The `ignore` crate supports custom ignore filenames via `add_custom_ignore_filename(".driftignore")`.
This means `.driftignore` files are hierarchical (like `.gitignore`) — a `.driftignore` in a
subdirectory applies to that subtree.

```
# .driftignore
node_modules/
dist/
build/
*.min.js
*.bundle.js
vendor/
__pycache__/
*.pyc
target/
.next/
coverage/
```

---

## 9. Error Handling (per AD6, 04-INFRASTRUCTURE.md)

```rust
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO error scanning {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("File too large: {path} ({size} bytes, max {max})")]
    FileTooLarge {
        path: PathBuf,
        size: u64,
        max: u64,
    },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("Config error: {message}")]
    Config { message: String },

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Scan cancelled")]
    Cancelled,
}
```

Errors are non-fatal at the file level — a single file failing to read/hash should not abort the
entire scan. Collect errors, continue scanning, report at the end.

---

## 10. Tracing / Observability (per AD10, 04-INFRASTRUCTURE.md)

```rust
use tracing::{info, warn, instrument, info_span};

#[instrument(skip(config, db), fields(root = %root.display()))]
pub fn scan(root: &Path, config: &ScanConfig, db: &DatabaseManager) -> Result<ScanDiff, ScanError> {
    let _span = info_span!("scan", root = %root.display()).entered();

    // Phase 1: Discovery
    let _discovery = info_span!("discovery").entered();
    let files = discover_files(root, config)?;
    info!(file_count = files.len(), "discovery complete");

    // Phase 2: Processing
    let _processing = info_span!("processing").entered();
    let diff = compute_diff(files, db)?;
    info!(
        added = diff.added.len(),
        modified = diff.modified.len(),
        removed = diff.removed.len(),
        unchanged = diff.unchanged.len(),
        "diff complete"
    );

    diff
}
```

Key metrics to emit:
- `scan_files_per_second` — overall throughput
- `discovery_duration_ms` — Phase 1 time
- `hashing_duration_ms` — Phase 2 time
- `cache_hit_rate` — % of files skipped via mtime check
- `files_skipped_too_large` — count of oversized files
- `files_skipped_ignored` — count of ignored files

---

## 11. Event Emissions (per D5, 04-INFRASTRUCTURE.md)

The scanner must emit events via `DriftEventHandler`. These are no-ops in standalone mode,
consumed by the bridge crate when Cortex is present.

```rust
pub trait DriftEventHandler: Send + Sync {
    fn on_scan_started(&self, _root: &Path, _file_count: Option<usize>) {}
    fn on_scan_progress(&self, _processed: usize, _total: usize) {}
    fn on_scan_complete(&self, _results: &ScanDiff) {}
    fn on_scan_error(&self, _error: &ScanError) {}
}
```

Emit `on_scan_progress` every 100 files (from audit). Use `AtomicU64` counter shared across
rayon workers, check modulo 100.

---

## 12. NAPI Interface (per 03-NAPI-BRIDGE.md)

The scanner follows the "compute + store in Rust, return summary" pattern.
Full results go to drift.db. Only a lightweight summary crosses the NAPI boundary.

### Primary NAPI Functions

```rust
/// Full scan — discovers files, hashes, computes diff, writes to drift.db
#[napi]
pub fn native_scan(root: String, options: ScanOptions) -> ScanSummary {
    // 1. Discover files (ignore crate)
    // 2. Hash + metadata (rayon)
    // 3. Diff against drift.db cache
    // 4. Update file_metadata table
    // 5. Return lightweight summary
    ScanSummary {
        files_total: 10_000,
        files_added: 42,
        files_modified: 15,
        files_removed: 3,
        files_unchanged: 9_940,
        duration_ms: 280,
        status: "complete".to_string(),
        languages: HashMap::from([
            ("typescript".into(), 5000),
            ("python".into(), 2000),
            // ...
        ]),
    }
}

/// Async variant with progress callback
#[napi]
pub fn native_scan_with_progress(
    root: String,
    options: ScanOptions,
    progress: ThreadsafeFunction<ProgressUpdate, NonBlocking>,
) -> AsyncTask<ScanTask> { ... }

/// Cancel a running scan
#[napi]
pub fn cancel_scan() { ... }
```

### What Crosses NAPI
- `ScanSummary` (counts, duration, status) — lightweight
- `ProgressUpdate` (processed count, total count) — periodic

### What Stays in Rust/SQLite
- Full file list
- Content hashes
- File metadata
- ScanDiff details (queryable via separate NAPI query functions)

### Query Functions (for MCP/CLI to read scan results)
```rust
#[napi]
pub fn query_changed_files(since: Option<i64>) -> Vec<FileChange> { ... }

#[napi]
pub fn query_file_metadata(path: String) -> Option<FileMetadata> { ... }
```

---

## 13. Cancellation (per audit A6/A21)

Global `AtomicBool` checked between files in the rayon processing phase:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

static SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);

// In rayon par_iter:
files.par_iter().try_for_each(|file| {
    if SCAN_CANCELLED.load(Ordering::Relaxed) {
        return Err(ScanError::Cancelled);
    }
    process_file(file)
})?;
```

Already-processed files are persisted. In-progress file is discarded.
Returns partial results with `status: "partial"`.

---

## 14. Performance Targets

From the audit:
- 10K files: scanner portion <300ms (scanner is ~10% of total <3s pipeline)
- 100K files: scanner portion <1.5s (scanner is ~10% of total <15s pipeline)
- Incremental (1 file changed): <100ms total

These are achievable:
- `ignore` crate can walk 100K+ files in <500ms on SSD
- xxh3 hashes typical source files (10-50KB) in microseconds
- mtime check is a single stat() call — nanoseconds

**macOS caveat**: APFS directory scanning is single-threaded at the kernel level.
Parallel walking helps with per-file work (hashing, metadata) but not directory enumeration.
This is a known limitation shared by ripgrep and fd.

---

## 15. Integration Points

### Scanner → Parsers
Scanner produces `ScanDiff`. Parsers consume `added` + `modified` lists to know which files
need (re)parsing. `unchanged` files use cached parse results (Moka + SQLite parse_cache table).

### Scanner → Storage
Scanner writes to `file_metadata` table in drift.db. Uses the batch writer pattern
(crossbeam bounded channel + dedicated writer thread) from 02-STORAGE.md for bulk updates
after a full scan.

### Scanner → Call Graph
Call graph builder receives the file list from the scanner. On incremental scans, it only
re-extracts call sites from `added` + `modified` files, removes edges for `removed` files.

### Scanner → NAPI → TS
TS layer (MCP server, CLI) calls `native_scan()` or `native_scan_with_progress()`.
Gets back a summary. Queries drift.db via NAPI query functions for details.

---

## 16. File Module Structure

```
crates/drift-core/src/scanner/
├── mod.rs          # Public API: scan(), scan_incremental()
├── walker.rs       # Phase 1: ignore crate WalkParallel discovery
├── hasher.rs       # Phase 2: xxh3 content hashing + metadata
├── diff.rs         # ScanDiff computation against drift.db cache
├── types.rs        # ScanConfig, ScanDiff, ScanEntry, ScanSummary, FileMetadata
└── errors.rs       # ScanError enum (thiserror)
```

---

## 17. Build Order

The scanner is the second thing built (after infrastructure). Exact sequence:

1. **Infrastructure** (already defined in 04-INFRASTRUCTURE.md):
   - `errors.rs` — ScanError + other subsystem error enums
   - `tracing` init — EnvFilter, per-subsystem spans
   - `DriftEventHandler` trait — with scan event methods
   - `config.rs` — DriftConfig with ScanConfig

2. **Scanner** (this system):
   - `types.rs` — data types
   - `errors.rs` — ScanError enum
   - `walker.rs` — file discovery via `ignore` crate
   - `hasher.rs` — xxh3 hashing
   - `diff.rs` — incremental detection
   - `mod.rs` — public API

3. **Storage** (file_metadata table):
   - Migration 001: create `file_metadata` table
   - Batch writer for bulk updates

4. **NAPI** (scanner bindings):
   - `native_scan()` + `native_scan_with_progress()`
   - Query functions for file metadata

---

## 18. Key API Surface

```rust
// ---- Public API ----

/// Full scan: discover all files, hash, compute diff against cache
pub fn scan(
    root: &Path,
    config: &ScanConfig,
    db: &DatabaseManager,
    event_handler: &dyn DriftEventHandler,
) -> Result<ScanDiff, ScanError>;

/// Incremental scan: only process files changed since last scan
/// (This is the same as scan() — the two-level mtime+hash strategy
/// makes every scan incremental by default)
pub fn scan_incremental(
    root: &Path,
    config: &ScanConfig,
    db: &DatabaseManager,
    event_handler: &dyn DriftEventHandler,
) -> Result<ScanDiff, ScanError>;

/// Discovery only: return file list without hashing or diffing
/// (useful for "how many files?" queries)
pub fn discover_files(
    root: &Path,
    config: &ScanConfig,
) -> Result<Vec<PathBuf>, ScanError>;

// ---- Types ----

pub struct ScanDiff {
    pub added: Vec<PathBuf>,
    pub modified: Vec<PathBuf>,
    pub removed: Vec<PathBuf>,
    pub unchanged: Vec<PathBuf>,
    pub errors: Vec<ScanError>,
    pub stats: ScanStats,
}

pub struct ScanStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub discovery_ms: u64,
    pub hashing_ms: u64,
    pub diff_ms: u64,
    pub cache_hit_rate: f64,      // % skipped via mtime
    pub files_skipped_large: usize,
    pub files_skipped_ignored: usize,
}

pub struct ScanEntry {
    pub path: PathBuf,
    pub content_hash: u64,        // xxh3
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub file_size: u64,
    pub language: Option<Language>,  // detected from file extension
}
```

---

## 19. v1 Feature Verification — Complete Gap Analysis

Cross-referenced against all v1 scanner documentation:
- `packages/core/src/scanner/` (7 TS files)
- `packages/core/src/services/scanner-service.ts` (ScannerService, ~1200 lines)
- `packages/core/src/services/detector-worker.ts` (DetectorWorker, ~350 lines)
- `00-overview/pipelines.md` (Pipeline 1: Full Scan)
- `.research/01-rust-core/RECAP.md` (v1 Rust scanner subsystem)
- `25-services-layer/scanner-service.md` + `detector-worker.md`
- `14-directory-map/packages-core.md` (full file listing)

### v1 Scanner Components (packages/core/src/scanner/)

| v1 File | v1 Feature | v2 Status | v2 Location |
|---------|-----------|-----------|-------------|
| `file-walker.ts` | Sequential TS file walker (slower fallback) | **DROPPED** — Rust-only in v2. No TS fallback needed. `ignore` crate is faster than any TS walker. | N/A |
| `native-scanner.ts` | Wrapper calling Rust NAPI `scan()` | **REPLACED** — TS calls `native_scan()` / `native_scan_with_progress()` directly. Thinner wrapper since Rust now writes to drift.db. | §12 NAPI Interface |
| `dependency-graph.ts` | Import/export dependency tracking from scanner | **MOVED** — Not scanner's job in v2. Dependency graph is built by the call graph builder (Level 1) from ParseResult data. Scanner only discovers files. | 05-CALL-GRAPH.md |
| `change-detector.ts` | Incremental change detection (content hash comparison) | **ABSORBED INTO RUST** — Two-level mtime+hash strategy in scanner's `diff.rs`. This was the #1 gap in v1 Rust scanner (noted in v1 docs: "Needs: incremental scanning added to Rust side"). Now core to v2 scanner. | §5 Incremental Detection |
| `default-ignores.ts` | Default ignore patterns (node_modules, dist, etc.) | **ABSORBED** — `ignore` crate handles .gitignore natively. Default patterns ship in `.driftignore` template created by `drift setup`. Extra ignores configurable via `scan.extra_ignore` in drift.toml. | §7 Configuration, §8 .driftignore |
| `worker-pool.ts` | Piscina worker thread pool management | **DROPPED** — Replaced by Rust's `rayon` thread pool. No TS worker threads needed. | §3 Two-Phase Architecture |
| `threaded-worker-pool.ts` | Alternative worker pool implementation | **DROPPED** — Same as above. Rayon handles all parallelism. | §3 |
| `file-processor-worker.ts` | Per-file processing in worker threads | **DROPPED** — Rust processes files directly via rayon `par_iter`. No worker dispatch overhead. | §3 |

### v1 ScannerService (packages/core/src/services/scanner-service.ts)

The v1 ScannerService was a ~1200-line orchestrator that combined scanning + detection + aggregation + outlier detection + manifest generation. In v2, these responsibilities are split:

| v1 ScannerService Feature | v2 Status | v2 Owner |
|--------------------------|-----------|----------|
| Worker pool creation + warmup | **DROPPED** — Rayon replaces Piscina | Scanner (rayon) |
| Task dispatch (1 task per file) | **REPLACED** — `rayon::par_iter` over file list | Scanner |
| Detector execution per file | **MOVED** — Detectors run in Rust unified analysis engine, not scanner | Unified Analysis Engine (Level 1) |
| Pattern aggregation across files | **MOVED** — Pattern aggregation is a post-detection step in Rust | Detector System / Pattern Aggregation (Level 2A) |
| Outlier detection (Z-score, IQR, Grubbs') | **MOVED** — Outlier detection runs after aggregation in Rust | Outlier Detection (Level 2A) |
| Manifest generation | **DROPPED** — SQLite materialized views replace manifests | Storage Gold layer |
| ScanResults assembly | **REPLACED** — Rust writes to drift.db, returns ScanSummary via NAPI | §12 NAPI Interface |
| Worker stats tracking | **REPLACED** — `tracing` spans + structured metrics | §10 Tracing |
| Graceful single-thread fallback | **DROPPED** — Rayon always works (degrades to 1 thread if needed) | N/A |
| Category filtering | **MOVED** — Detector registry handles category filtering | Detector System (Level 1) |
| Critical-only mode | **MOVED** — Detector registry handles critical-only filtering | Detector System (Level 1) |
| ProjectContext passing | **REPLACED** — Rust has direct access to all data via drift.db | N/A |
| Error collection (non-fatal) | **KEPT** — `ScanDiff.errors: Vec<ScanError>` collects per-file errors | §9 Error Handling, §18 API |

### v1 DetectorWorker (packages/core/src/services/detector-worker.ts)

| v1 DetectorWorker Feature | v2 Status | v2 Owner |
|--------------------------|-----------|----------|
| Language detection from extension | **MOVED** — Parser manager handles language detection | Parsers (Level 0) |
| Detector loading + caching | **MOVED** — Detector registry in Rust, static initialization | Detector System (Level 1) |
| DetectionContext building | **MOVED** — Rust builds context internally | Unified Analysis Engine |
| Metadata preservation (endLine, endColumn, isOutlier, matchedText) | **KEPT** — All metadata preserved in Rust types and drift.db | Detector System types |
| 25+ language extension mapping | **MOVED** — Parser manager owns extension→language mapping | Parsers |

### v1 Pipeline Features (from 00-overview/pipelines.md)

| Pipeline 1 Step | v2 Status | Notes |
|----------------|-----------|-------|
| 1. Resolve project root | **KEPT** — Config system resolves root | Configuration (Level 0) |
| 2. File discovery + ignore + max-file-size + incremental | **KEPT** — All in v2 scanner | §3, §5, §7, §8 |
| 3. Parsing per file | **KEPT** — Parsers consume scanner output | Parsers (Level 0) |
| 4. Detection per file (parallel) | **KEPT** — Rayon parallelism in Rust | Unified Analysis Engine |
| 5. Aggregation across files | **KEPT** — In Rust | Pattern Aggregation (Level 2A) |
| 6. Confidence scoring | **UPGRADED** — Bayesian (Beta posterior + momentum) replaces static formula | Confidence Scoring (Level 2A) |
| 7. Pattern storage | **UPGRADED** — drift.db (SQLite) replaces JSON shards | Storage (Level 0) |
| 8. Call graph build (optional) | **KEPT** — Separate system, triggered after scan | Call Graph (Level 1) |
| 9. Boundary scan (optional) | **KEPT** — Separate system | Boundary Detection (Level 1) |
| 10. Contract scan (optional) | **KEPT** — Separate system | Contract Tracking (Level 2C) |
| 11. Manifest generation (optional) | **DROPPED** — SQLite Gold layer replaces | Storage |
| 12. Finalization (history, audit, materialize) | **KEPT** — Post-scan refresh of Gold layer | Storage, Audit (Level 3) |

### v1 Features NOT in v2 Prep Doc (Gaps Found)

These v1 features were missing from the original v2 scanner prep doc and need to be accounted for:

**1. Language Detection in Scanner Output**
v1 scanner returned detected language per file. v2 prep doc only returns paths + hashes.
**Resolution**: Add `detected_language: Option<Language>` to `ScanEntry`. The scanner already
reads file extensions during discovery — detecting language is trivial (extension mapping).
This avoids the parser having to re-derive language from the path.

**2. Total Size Tracking**
v1 returned `totalSize` (sum of all file sizes) in scan stats.
**Resolution**: Already in v2 prep doc as `ScanStats.total_size_bytes`. ✅ Covered.

**3. Languages Found Summary**
v1 returned a summary of which languages were found and file counts per language.
**Resolution**: Add to `ScanStats`:
```rust
pub languages_found: FxHashMap<Language, usize>,  // language → file count
```

**4. Verbose Mode**
v1 had `verbose: boolean` in ScannerServiceConfig for detailed logging.
**Resolution**: Handled by `tracing` crate's `DRIFT_LOG` env var. `DRIFT_LOG=scanner=debug`
gives verbose scanner output. No need for a separate verbose flag. ✅ Covered differently.

**5. Incremental Flag**
v1 had explicit `incremental: boolean` flag. v2 scanner is always incremental by design
(two-level mtime+hash). But there should be a way to force a full rescan.
**Resolution**: Add `force_full_scan: bool` to `ScanConfig`. When true, skip mtime check
and re-hash everything. Useful after git operations that change many mtimes.

**6. Default Ignore Patterns**
v1 had `default-ignores.ts` with hardcoded patterns (node_modules, .git, dist, build, etc.).
The `ignore` crate handles .gitignore but NOT default patterns for non-git-ignored directories.
**Resolution**: The scanner should have built-in default ignores that apply even without
a .gitignore or .driftignore file:
```rust
const DEFAULT_IGNORES: &[&str] = &[
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
    "vendor",       // PHP, Go
    ".venv",
    "venv",
    ".tox",
    ".mypy_cache",
    "bin",          // C#, Java build output
    "obj",          // C# build output
];
```
These are applied via `ignore::overrides::OverrideBuilder` as default exclusions,
overridable by config.

**7. File Count Per Language in ScanSummary (NAPI)**
v1's NAPI `JsScanResult` included language breakdown. v2's `ScanSummary` only has counts.
**Resolution**: Add `languages: HashMap<String, u32>` to the NAPI `ScanSummary` struct.
Small data, useful for CLI display ("Found 5,000 TypeScript, 2,000 Python files").

---

## 20. Updated ScanEntry and ScanStats (Post-Verification)

After v1 verification, the types need these additions:

```rust
pub struct ScanEntry {
    pub path: PathBuf,
    pub content_hash: u64,           // xxh3
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
    pub file_size: u64,
    pub language: Option<Language>,  // NEW: detected from extension
}

pub struct ScanStats {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub discovery_ms: u64,
    pub hashing_ms: u64,
    pub diff_ms: u64,
    pub cache_hit_rate: f64,
    pub files_skipped_large: usize,
    pub files_skipped_ignored: usize,
    pub languages_found: FxHashMap<Language, usize>,  // NEW: language breakdown
}

pub struct ScanConfig {
    pub max_file_size: Option<u64>,
    pub threads: Option<usize>,
    pub extra_ignore: Vec<String>,
    pub follow_symlinks: Option<bool>,
    pub compute_hashes: Option<bool>,
    pub force_full_scan: Option<bool>,  // NEW: skip mtime, re-hash everything
    pub skip_binary: Option<bool>,      // NEW: skip binary files (default true)
}
```

---

## 21. Open Items / Decisions Still Needed

1. **Symlink handling**: Default is `follow_symlinks = false`. The `ignore` crate supports
   `follow_links(true)` but this can cause infinite loops with circular symlinks.
   Need to decide: follow with cycle detection, or never follow?
   Recommendation: never follow (matches git behavior).

2. **Binary file detection**: The `ignore` crate can skip binary files via `is_binary()` heuristic
   (checks first 8KB for null bytes). Should the scanner skip binaries automatically?
   Recommendation: yes, skip by default. Source code analysis on binaries is meaningless.
   Make configurable: `scan.skip_binary = true`.

3. **Watch mode**: The audit mentions `drift scan --watch`. This needs a file watcher
   (likely `notify` crate) integrated with the scanner for continuous incremental scanning.
   This is a separate system that calls `scan_incremental()` on file change events.
   Not part of the initial scanner build — add as a follow-up.

4. **Parallel hashing I/O saturation**: On HDD (not SSD), parallel file reads can cause
   seek thrashing. Consider a configurable I/O concurrency limit separate from CPU thread count.
   For v2 initial build: ignore this (target is SSD). Revisit if users report HDD issues.

5. **Content hash storage format**: xxh3 produces a `u64` (8 bytes). Store as BLOB in SQLite
   for compact storage, or as INTEGER for easier querying? BLOB is more correct (hash is not
   a number, it's an opaque identifier). But INTEGER is simpler and SQLite handles 64-bit ints.
   Recommendation: store as INTEGER (SQLite INTEGER is 8 bytes, same as xxh3 output).

6. **Dependency graph ownership**: v1 had `dependency-graph.ts` in the scanner directory.
   v2 moves this to the call graph builder. Confirm this is the right boundary — the scanner
   should NOT build import/export graphs. It discovers files. Period.
