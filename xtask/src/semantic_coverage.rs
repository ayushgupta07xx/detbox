//! How much of the real corpus `semantic_view` can actually model — konflux M2.
//!
//! K1 asks whether the bytes come back. This asks a harder question: of the
//! files konflux can *parse*, how many can it *understand* well enough to diff?
//!
//! The number matters because [`core_formats::semantic`] refuses rather than
//! guesses (ADR-012), and a layer that refuses everything is indistinguishable
//! from one that does not exist. Publishing the refusal reasons, grouped and
//! counted, turns "not modelled yet" from an excuse into a work queue ordered
//! by how much real config each item unblocks.
//!
//! In `xtask` rather than a `#[test]` for the same reason as `corpus-k1`: the
//! corpus is fetched, not committed (ADR-004), so a test would either fail on
//! every machine that has not fetched it, or skip — and a skip that reports
//! green is the vacuity invariant V4 forbids.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use core_formats::{Format, Json, Yaml};

/// Walk the corpus and report per-format semantic coverage.
///
/// # Errors
///
/// Errors when the corpus is absent. Never on a refusal: a refusal is the
/// measurement, not a failure.
pub(crate) fn explain(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let yaml = Yaml;
    let cst = yaml
        .parse(&bytes)
        .map_err(|r| format!("does not parse ({} diagnostics)", r.diagnostics().len()))?;
    match yaml.semantic_view(&cst) {
        Ok(_) => Ok("modelled\n".to_string()),
        Err(why) => Ok(format!("refused: {}\n", why.reason)),
    }
}

pub(crate) fn run(root: &Path) -> Result<String, String> {
    let corpora = root.join("corpora");
    let mut files: Vec<PathBuf> = Vec::new();
    collect(&corpora, &mut files);
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "no corpus under {} — run corpora/fetch.sh first.\n\
             An error rather than a skip: a coverage run that measured nothing\n\
             and reported success is the vacuity invariant V4 forbids.",
            corpora.display()
        ));
    }

    let yaml = Yaml;
    let json = Json;
    let mut out = String::from("semantic_view coverage over the corpus — konflux M2\n\n");
    let mut grand_total = 0usize;
    let mut grand_modelled = 0usize;

    for format in [&yaml as &dyn Format, &json as &dyn Format] {
        let mut total = 0usize;
        let mut modelled = 0usize;
        let mut unparsed = 0usize;
        // BTreeMap, not HashMap: this ordering reaches an output (§9.5).
        // Count plus one example path: a work queue nobody can reproduce is a
        // list of complaints. The example is the first in sorted order, so it
        // is the same one on every machine.
        let mut reasons: BTreeMap<&'static str, (usize, String)> = BTreeMap::new();

        for path in &files {
            let extension = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !format.extensions().contains(&extension.as_str()) {
                continue;
            }
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            total += 1;
            let Ok(cst) = format.parse(&bytes) else {
                unparsed += 1;
                continue;
            };
            match format.semantic_view(&cst) {
                Ok(_) => modelled += 1,
                Err(why) => {
                    let slot = reasons
                        .entry(why.reason)
                        .or_insert_with(|| (0, relative(root, path)));
                    slot.0 += 1;
                }
            }
        }

        if total == 0 {
            continue;
        }
        grand_total += total;
        grand_modelled += modelled;

        let _ = writeln!(
            out,
            "  {:<6} {modelled}/{total} modelled ({:.1}%)",
            format.name(),
            percent(modelled, total)
        );
        if unparsed > 0 {
            let _ = writeln!(
                out,
                "         {unparsed} did not parse (not a view problem)"
            );
        }
        // Sorted by count descending, then by reason, so the work queue is
        // ordered by how much corpus each item unblocks — and is deterministic.
        let mut ranked: Vec<(&&str, &(usize, String))> = reasons.iter().collect();
        ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(b.0)));
        for (reason, (count, example)) in ranked {
            let _ = writeln!(out, "         {count:>4}  {reason}");
            let _ = writeln!(out, "               e.g. {example}");
        }
        out.push('\n');
    }

    let _ = writeln!(
        out,
        "  TOTAL  {grand_modelled}/{grand_total} ({:.1}%)",
        percent(grand_modelled, grand_total)
    );
    out.push_str(
        "\n  Refusals are the honest answer, not a bug (ADR-012): a view we cannot\n  \
         build is one konflux declines to diff rather than diffing wrongly.\n",
    );
    Ok(out)
}

/// Repo-relative, `/`-separated (§9.5).
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect::<Vec<_>>()
        .join("/")
}

fn percent(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        return 0.0;
    }
    // Counts are in the thousands; f64 has ample mantissa here.
    #[allow(clippy::cast_precision_loss)]
    {
        part as f64 / whole as f64 * 100.0
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}
