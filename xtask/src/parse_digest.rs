//! K3 for the real parser: a deterministic digest of every golden case.
//!
//! MASTER_PLAN §8 requires a "double-build + double-run output-hash compare"
//! gate, and ADR-003 said that at konflux M1 it would additionally run the real
//! parse/serialize pair rather than only the repo scaffold report.
//!
//! This emits, for every checked-in golden case, the format, the accept/reject
//! verdict, and the SHA-256 of the serialised tree. CI builds `xtask` twice and
//! compares the two runs byte for byte.
//!
//! It reads the **committed** golden suites rather than the fetched corpus, so
//! it runs in the determinism job without a network fetch. The corpus half of K1
//! is `corpus-k1`, in the job that does fetch.
//!
//! What this catches that K1 alone does not: a parser that is *correct* but not
//! *deterministic* — a `HashMap` whose iteration order reaches the output, a
//! sort that is not stable, a path separator that differs on Windows. K1 asks
//! whether the bytes came back; K3 asks whether they come back the same way
//! every time, on every platform.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use core_formats::{Format, Json, Yaml};

use crate::sha256;

/// Render the digest. Sorted by case path, so two runs agree byte for byte.
///
/// # Errors
///
/// Errors when no golden case is found: a determinism report over nothing would
/// be identical across runs and prove nothing (invariant V4).
pub(crate) fn run(root: &Path) -> Result<String, String> {
    let golden = root.join("crates/core-formats/tests/golden");
    let mut cases: Vec<PathBuf> = Vec::new();
    collect(&golden, &mut cases);
    cases.sort();

    if cases.is_empty() {
        return Err(format!(
            "no golden cases under {} — a determinism digest over nothing is\n\
             identical across runs and proves nothing (core-verify invariant V4).",
            golden.display()
        ));
    }

    let yaml = Yaml;
    let json = Json;
    let mut out = String::new();
    let _ = writeln!(out, "# K3 digest — serialize(parse(x)) per golden case");
    let _ = writeln!(out, "# cases: {}", cases.len());

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
        let name = relative(root, path);
        match format.parse(&bytes) {
            Ok(cst) => {
                let _ = writeln!(
                    out,
                    "{name}\t{}\taccept\t{}",
                    format.name(),
                    sha256::hex(&format.serialize(&cst))
                );
            }
            Err(report) => {
                // The diagnostic text is part of the output contract: a parser
                // whose *message* varies between runs is as nondeterministic as
                // one whose bytes do, and `--json` output carries these.
                let _ = writeln!(
                    out,
                    "{name}\t{}\treject\t{}",
                    format.name(),
                    sha256::hex(report.to_string().as_bytes())
                );
            }
        }
    }
    Ok(out)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
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
