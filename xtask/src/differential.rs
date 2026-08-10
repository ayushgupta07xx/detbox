//! The differential runner — konflux **M2**, arming `gate/differential`.
//!
//! # Why this is not "konflux versus diff3"
//!
//! MILESTONES says *"ours vs `diff3`/`git diff` on the corpus"*, and taken
//! literally that comparison is ill-posed: a line differ and a structural
//! differ do not compute the same function, so there is no agreement set to
//! measure. `git diff` says *these bytes differ*; konflux says *this key
//! changed, and it means something*. Neither answer refutes the other, and a
//! runner that "compared" them would be inventing a scoring rule and calling it
//! evidence.
//!
//! What git *is* a sound oracle for is the byte-level question, and that turns
//! out to be enough to build a real differential on:
//!
//! - **bytes identical ⟹ konflux must report nothing.** If it reports a change
//!   between a file and itself, the diff is not a function of its inputs.
//! - **a known semantic edit ⟹ konflux must report it.** Here the oracle is
//!   ground truth we constructed rather than a second tool, which is *stronger*
//!   than an external comparison: we know exactly what changed, so silence is
//!   unambiguously a lost edit rather than a difference of opinion.
//!
//! A lost edit is the worst failure this tool has. A merge driver that does not
//! notice a changed value will drop it, silently, in someone's cluster config.
//!
//! # The mutation
//!
//! One top-level key appended, `zzz_konflux_probe: 1`. Deterministic, valid on
//! any mapping-rooted document, and impossible to confuse with existing
//! content. Documents that are not mapping-rooted, or that stop being modelled
//! once mutated, are **skipped and counted** — a runner that quietly narrows
//! its own corpus is reporting on a sample it chose.
//!
//! In `xtask` for the same reason as `corpus-k1`: the corpus is fetched, not
//! committed (ADR-004).

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use core_formats::{Format, Json, SemanticNode, Yaml};

/// Appended to make a document differ semantically in a way we chose.
const PROBE: &str = "zzz_konflux_probe: 1\n";
const PROBE_JSON_KEY: &str = "zzz_konflux_probe";

#[derive(Default)]
struct Tally {
    checked: usize,
    skipped_unmodelled: usize,
    skipped_not_a_mapping: usize,
    skipped_mutation_unreadable: usize,
    skipped_mutation_changed_shape: usize,
    /// Files where a diff against *itself* was not empty.
    unstable: Vec<String>,
    /// Files where a known added key produced no semantic change. Lost edits.
    lost: Vec<String>,
}

/// Run the differential over every corpus file konflux can model.
///
/// # Errors
///
/// Errors when the corpus is absent, or when any divergence is found. A lost
/// edit is a release blocker, not a statistic.
pub(crate) fn run(root: &Path) -> Result<String, String> {
    let corpora = root.join("corpora");
    let mut files: Vec<PathBuf> = Vec::new();
    collect(&corpora, &mut files);
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "no corpus under {} — run corpora/fetch.sh first.\n\
             An error rather than a skip: a differential run that compared\n\
             nothing and reported success is the vacuity invariant V4 forbids.",
            corpora.display()
        ));
    }

    let mut tally = Tally::default();
    for path in &files {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let name = relative(root, path);
        match extension.as_str() {
            "yaml" | "yml" => check(&Yaml, &bytes, &name, &mut tally, mutate_yaml),
            "json" => check(&Json, &bytes, &name, &mut tally, mutate_json),
            _ => {}
        }
    }

    finish(report(&tally), &tally)
}

/// Both halves of the differential for one file.
fn check<F: Format>(
    format: &F,
    bytes: &[u8],
    name: &str,
    tally: &mut Tally,
    mutate: fn(&[u8]) -> Option<Vec<u8>>,
) {
    // Only files konflux claims to understand are in scope. A refusal is a
    // measured outcome (ADR-013), not a differential failure.
    let Ok(cst) = format.parse(bytes) else {
        return;
    };
    let Ok(view) = format.semantic_view(&cst) else {
        tally.skipped_unmodelled += 1;
        return;
    };
    if !matches!(view, SemanticNode::Mapping(_)) {
        tally.skipped_not_a_mapping += 1;
        return;
    }

    // Half one: a file against itself. git would say "identical"; so must we.
    match konflux::diff(format, bytes, bytes) {
        Ok(report) if report.changes.is_empty() => {}
        _ => tally.unstable.push(name.to_string()),
    }

    // Half two: a semantic edit we chose, so silence is unambiguous.
    let Some(mutated) = mutate(bytes) else {
        tally.skipped_mutation_unreadable += 1;
        return;
    };
    debug_assert_ne!(bytes, mutated.as_slice(), "the mutation changed no bytes");
    let Ok(mutated_cst) = format.parse(&mutated) else {
        tally.skipped_mutation_unreadable += 1;
        return;
    };
    // The mutated document must still be mapping-rooted. A file ending in a
    // trailing `---` turns an appended key into a *second document*, which is a
    // shape change rather than an added key — konflux reporting a replacement
    // there is correct, and counting it as a lost edit would be the runner
    // blaming the tool for the runner's own mutation. Found on the first pass:
    // 8 of the 9 "lost edits" were istio manifests ending in `---`.
    if !matches!(
        format.semantic_view(&mutated_cst),
        Ok(SemanticNode::Mapping(_))
    ) {
        tally.skipped_mutation_changed_shape += 1;
        return;
    }

    tally.checked += 1;
    let noticed = konflux::diff(format, bytes, &mutated).is_ok_and(|report| {
        report.changes.iter().any(|change| {
            change.significance == konflux::Significance::Semantic
                && change.path.contains(PROBE_JSON_KEY)
        })
    });
    if !noticed {
        tally.lost.push(name.to_string());
    }
}

/// Append a top-level key. Valid on any mapping-rooted YAML document, provided
/// the file ends on a line boundary — if it does not, adding one would itself
/// be a byte change of a different kind, so that case is declined.
fn mutate_yaml(bytes: &[u8]) -> Option<Vec<u8>> {
    if !bytes.ends_with(b"\n") {
        return None;
    }
    let mut out = bytes.to_vec();
    out.extend_from_slice(PROBE.as_bytes());
    Some(out)
}

/// Insert a key after the opening brace, which is where a JSON object will
/// always accept one.
fn mutate_json(bytes: &[u8]) -> Option<Vec<u8>> {
    let text = std::str::from_utf8(bytes).ok()?;
    let at = text.find('{')?;
    let rest = text.get(at + 1..)?;
    // An empty object takes no comma; anything else does.
    let comma = if rest.trim_start().starts_with('}') {
        ""
    } else {
        ","
    };
    let head = text.get(..=at)?;
    Some(format!("{head}\"{PROBE_JSON_KEY}\": 1{comma}{rest}").into_bytes())
}

fn report(tally: &Tally) -> String {
    let mut out = String::from("differential — konflux M2\n\n");
    let _ = writeln!(
        out,
        "  {} file(s) checked: diffed against themselves, then against a known added key",
        tally.checked
    );
    let _ = writeln!(
        out,
        "  skipped: {} unmodelled, {} not mapping-rooted, {} unreadable once mutated, \
{} reshaped by the mutation",
        tally.skipped_unmodelled,
        tally.skipped_not_a_mapping,
        tally.skipped_mutation_unreadable,
        tally.skipped_mutation_changed_shape
    );
    out.push_str(
        "\n  The skips are printed because a runner that quietly narrows its own\n  \
         corpus is reporting on a sample it chose.\n",
    );
    out
}

/// A lost edit is a release blocker, not a statistic.
fn finish(out: String, tally: &Tally) -> Result<String, String> {
    if tally.unstable.is_empty() && tally.lost.is_empty() {
        return Ok(out);
    }
    let listed = |what: &[String]| -> String {
        what.iter()
            .take(20)
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n    ")
    };
    let mut message = out;
    if !tally.unstable.is_empty() {
        let _ = write!(
            message,
            "\nNOT A FUNCTION OF ITS INPUTS: {} file(s) differ from themselves:\n    {}\n",
            tally.unstable.len(),
            listed(&tally.unstable)
        );
    }
    if !tally.lost.is_empty() {
        let _ = write!(
            message,
            "\nLOST EDIT: {} file(s) where an added top-level key produced no\n\
             semantic change:\n    {}\n\n\
             This is the worst failure konflux has. A merge driver that does not\n\
             notice a changed key will drop it, silently, in someone's cluster\n\
             config. Minimise one, file it as a golden case, then fix it.\n",
            tally.lost.len(),
            listed(&tally.lost)
        );
    }
    Err(message)
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
        } else if path.is_file() {
            out.push(path);
        }
    }
}
