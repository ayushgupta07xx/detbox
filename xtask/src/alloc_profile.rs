//! Allocation profile: a **deterministic** performance gate.
//!
//! # Why timings cannot gate
//!
//! `benches/baselines/linux-x86_64.tsv` was calibrated from a CI run on `main`,
//! and the very next run flagged a 43% regression on a parser that had not
//! changed by a byte. Six runs of data explain it:
//!
//! | | within-run 95% CI | between-run spread |
//! |---|---|---|
//! | criterion's own measurement | ±0.6% – ±1.4% | — |
//! | the same benchmark, run to run | — | **+38% / −20%** |
//!
//! Criterion is measuring precisely. The thing being measured is not stable.
//! And the failing run gave the *fastest* result of six for three benchmarks
//! and the *slowest* for a fourth, in one job on one host — so it is not simply
//! a slow machine, which would have moved everything together.
//!
//! No tolerance survives that. Wide enough not to cry wolf (±45%) is wide enough
//! to miss any regression worth catching, and a gate that cries wolf is worse
//! than no gate: it teaches everyone to ignore red.
//!
//! # What gates instead
//!
//! Allocation counts. They are **exact**: the same input through the same code
//! allocates the same number of times, on every run, on every platform, at every
//! optimisation level. No tolerance is needed because there is no noise — the
//! baseline is compared for equality, like a golden file.
//!
//! They are also the right proxy for this workload. A lossless parser's cost is
//! dominated by building the tree, and MASTER_PLAN §0 puts deterministic,
//! machine-verifiable measures above probabilistic ones. Timings are still
//! measured and published (§12 requires the benchmark table); they simply do not
//! decide whether CI is red.
//!
//! Inputs are the committed golden suites — the same bytes the K1 gate uses, so
//! the two views describe the same work.

// The workspace denies `unsafe_code`. `GlobalAlloc` cannot be implemented
// without it — the trait is unsafe by definition, because the allocator is
// trusted by everything above it. This is the only `unsafe` in the workspace.
//
// What it does: increment two counters, then delegate every operation verbatim
// to `System`. It allocates nothing itself, changes no pointer, and makes no
// assumption `System` does not already make. The counters are `Relaxed` atomics
// because they are statistics, not synchronisation.
#![allow(unsafe_code)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

use core_formats::{Format, Json, Yaml};

static ALLOCS: AtomicUsize = AtomicUsize::new(0);
static BYTES: AtomicUsize = AtomicUsize::new(0);
static COUNTING: AtomicUsize = AtomicUsize::new(0);

/// Counts allocations, but only while `measure` has switched it on.
///
/// Gated rather than always-on so that reading directories, formatting the
/// report and everything else this binary does stays out of the numbers.
pub(crate) struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Relaxed) != 0 {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(layout.size(), Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Relaxed) != 0 {
            ALLOCS.fetch_add(1, Relaxed);
            BYTES.fetch_add(new_size.saturating_sub(layout.size()), Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn measure<T>(body: impl FnOnce() -> T) -> (usize, usize) {
    let allocs = ALLOCS.load(Relaxed);
    let bytes = BYTES.load(Relaxed);
    COUNTING.store(1, Relaxed);
    let value = body();
    COUNTING.store(0, Relaxed);
    drop(value);
    (ALLOCS.load(Relaxed) - allocs, BYTES.load(Relaxed) - bytes)
}

/// Profile parse+serialize over every golden case.
///
/// # Errors
///
/// Errors when no golden case is found: a profile over nothing is identical
/// across runs and proves nothing (invariant V4).
pub(crate) fn run(root: &Path) -> Result<String, String> {
    let golden = root.join("crates/core-formats/tests/golden");
    let mut cases: Vec<PathBuf> = Vec::new();
    collect(&golden, &mut cases);
    cases.sort();

    if cases.is_empty() {
        return Err(format!(
            "no golden cases under {} — an allocation profile over nothing is\n\
             identical across runs and proves nothing (invariant V4).",
            golden.display()
        ));
    }

    let yaml = Yaml;
    let json = Json;
    let mut totals: Vec<(&'static str, usize, usize, usize)> =
        vec![("json", 0, 0, 0), ("yaml", 0, 0, 0)];

    for path in &cases {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let format: &dyn Format = if json.extensions().contains(&extension.as_str()) {
            &json
        } else {
            &yaml
        };
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let (allocs, allocated) =
            measure(|| format.parse(&bytes).map(|cst| format.serialize(&cst)).ok());
        if let Some(row) = totals.iter_mut().find(|(n, ..)| *n == format.name()) {
            row.1 += 1;
            row.2 += allocs;
            row.3 += allocated;
        }
    }

    let mut out = String::new();
    let _ = writeln!(out, "#! schema = 1");
    let _ = writeln!(
        out,
        "# Allocations for parse+serialize over the committed golden cases."
    );
    let _ = writeln!(out, "# format\tcases\tallocations\tbytes");
    for (name, cases, allocs, bytes) in &totals {
        let _ = writeln!(out, "{name}\t{cases}\t{allocs}\t{bytes}");
    }
    Ok(out)
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.is_file() && path.file_name().is_some_and(|n| n != "README.md") {
            out.push(path);
        }
    }
}

/// Compare a profile against its recorded baseline, by **equality**.
///
/// # Errors
///
/// Errors on any difference. There is no tolerance because there is no noise:
/// a changed number means the parser's allocation behaviour changed, which is
/// either an improvement worth recording or a regression worth explaining.
pub(crate) fn compare(profile: &str, baseline_path: &Path) -> Result<String, String> {
    let expected = std::fs::read_to_string(baseline_path)
        .map_err(|e| format!("cannot read {}: {e}", baseline_path.display()))?;

    let rows = |text: &str| -> Vec<String> {
        text.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(str::to_string)
            .collect()
    };
    let (got, want) = (rows(profile), rows(&expected));

    if got == want {
        let mut out = String::from("allocation profile matches the baseline exactly\n");
        for row in &got {
            let _ = writeln!(out, "  {row}");
        }
        return Ok(out);
    }

    let mut out = String::from(
        "ALLOCATION PROFILE CHANGED\n\n\
         This gate compares for equality, not within a tolerance: allocation\n\
         counts are exact, so a difference is a real change in behaviour rather\n\
         than measurement noise.\n\n",
    );
    let _ = writeln!(out, "  expected (baseline):");
    for row in &want {
        let _ = writeln!(out, "    {row}");
    }
    let _ = writeln!(out, "  measured:");
    for row in &got {
        let _ = writeln!(out, "    {row}");
    }
    let _ = writeln!(
        out,
        "\n  Fewer allocations is an improvement: record the new numbers.\n  \
         More is a regression: explain it, then record them. Either way the\n  \
         baseline change is reviewed (§8), and golden-guard covers this path."
    );
    Err(out)
}
