//! Corpus survey: which YAML constructs actually appear, and how often.
//!
//! MILESTONES M1 asks for golden suites *"seeded from the 1,000-file corpus."*
//! The tempting reading is to copy corpus files in as golden cases. That is the
//! wrong one: it vendors third-party bytes the fetch-script design exists to
//! avoid (ADR-004), and it produces a suite whose coverage nobody can state.
//!
//! The useful reading is that the corpus decides *what the cases must cover*.
//! This module counts the constructs, so the golden suite can be justified by
//! frequency instead of by imagination — and so a construct that shows up in
//! 40% of real Helm charts cannot be quietly missing from the oracle.
//!
//! Detection is byte-level and deliberately crude: it is a **coverage
//! argument**, not a parser, and it runs before any parser exists. It
//! over-counts in places (a `#` inside a quoted scalar reads as a comment).
//! That bias is safe in one direction only — it can claim a construct is
//! present when it is not, never absent when it is — so a case list derived
//! from it is a superset of what the corpus needs.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// One construct we care about, and the reason a K1 oracle has to cover it.
struct Probe {
    name: &'static str,
    why: &'static str,
    hit: fn(&[u8], &str) -> bool,
}

/// Constructs are ordered by name so the report is stable (MASTER_PLAN §9.5).
const PROBES: &[Probe] = &[
    Probe {
        name: "anchor",
        why: "&name must survive; expanding it is a K1 violation",
        hit: |_, t| contains_token(t, '&'),
    },
    Probe {
        name: "alias",
        why: "*name must survive unexpanded",
        hit: |_, t| contains_token(t, '*'),
    },
    Probe {
        name: "merge-key",
        why: "<<: is a merge, and resolving it loses the source bytes",
        hit: |_, t| t.contains("<<:"),
    },
    Probe {
        name: "block-scalar",
        why: "| and > carry indentation and chomping indicators verbatim",
        hit: |_, t| {
            t.lines()
                .any(|l| l.trim_end().ends_with('|') || l.trim_end().ends_with('>'))
                || t.contains("|-")
                || t.contains(">-")
                || t.contains("|+")
        },
    },
    Probe {
        name: "flow-collection",
        why: "{a: 1} and [1, 2] must not be re-rendered as block style",
        hit: |_, t| t.contains('{') || t.contains('['),
    },
    Probe {
        name: "go-template",
        why: "Helm's {{ }} is not YAML at all; the grammar must preserve it verbatim",
        hit: |_, t| t.contains("{{"),
    },
    Probe {
        name: "comment",
        why: "the single most-lost byte class in every existing tool",
        hit: |_, t| t.lines().any(|l| l.trim_start().starts_with('#')),
    },
    Probe {
        name: "trailing-comment",
        why: "a comment after a value, where column alignment is meaningful to humans",
        hit: |_, t| {
            t.lines()
                .any(|l| !l.trim_start().starts_with('#') && l.contains(" #"))
        },
    },
    Probe {
        name: "multi-document",
        why: "--- and ... separators, and the bytes between them",
        hit: |_, t| {
            t.lines()
                .any(|l| l == "---" || l.starts_with("--- ") || l == "...")
        },
    },
    Probe {
        name: "directive",
        why: "%YAML and %TAG lines precede the document and are easily dropped",
        hit: |_, t| t.lines().any(|l| l.starts_with('%')),
    },
    Probe {
        name: "tag",
        why: "!!str and !Custom are exactly the exotic tags §3.1 sends to verbatim nodes",
        hit: |_, t| t.contains("!!") || t.lines().any(|l| l.contains(": !")),
    },
    Probe {
        name: "single-quoted",
        why: "quoting style is a byte-level choice; normalising it breaks K1",
        hit: |_, t| t.contains('\''),
    },
    Probe {
        name: "double-quoted",
        why: "same, plus escape sequences that must not be re-encoded",
        hit: |_, t| t.contains('"'),
    },
    Probe {
        name: "empty-value",
        why: "`key:` with nothing after it is not `key: null`",
        hit: |_, t| t.lines().any(|l| l.trim_end().ends_with(':')),
    },
    Probe {
        name: "crlf",
        why: "line endings are bytes; converting them is a silent corruption",
        hit: |b, _| b.windows(2).any(|w| w == b"\r\n"),
    },
    Probe {
        name: "tab",
        why: "tabs are illegal for YAML indentation but appear inside scalars",
        hit: |b, _| b.contains(&b'\t'),
    },
    Probe {
        name: "trailing-whitespace",
        why: "invisible, meaningless, and must survive anyway",
        hit: |_, t| t.lines().any(|l| l.ends_with(' ') || l.ends_with('\t')),
    },
    Probe {
        name: "no-final-newline",
        why: "the classic off-by-one at EOF",
        hit: |b, _| !b.is_empty() && b.last() != Some(&b'\n'),
    },
    Probe {
        name: "blank-line",
        why: "vertical spacing is authorial intent, not noise",
        hit: |_, t| t.lines().any(str::is_empty),
    },
    Probe {
        name: "bom",
        why: "a UTF-8 BOM is three bytes that belong to the file",
        hit: |b, _| b.starts_with(&[0xef, 0xbb, 0xbf]),
    },
    Probe {
        name: "non-ascii",
        why: "multi-byte scalars must not be re-encoded or escaped",
        hit: |b, _| b.iter().any(|c| *c >= 0x80),
    },
    Probe {
        name: "non-utf8",
        why: "parse must not panic, and the bytes must round-trip regardless",
        hit: |b, _| std::str::from_utf8(b).is_err(),
    },
    Probe {
        name: "indent-not-2",
        why: "3- and 4-space indentation is common and must not be reflowed",
        hit: |_, t| {
            t.lines().any(|l| {
                let n = l.len() - l.trim_start_matches(' ').len();
                n > 0 && !n.is_multiple_of(2)
            })
        },
    },
    Probe {
        name: "deep-nesting",
        why: "8+ levels: where a recursive serializer meets the stack",
        hit: |_, t| {
            t.lines()
                .any(|l| l.len() - l.trim_start_matches(' ').len() >= 16)
        },
    },
    Probe {
        name: "long-line",
        why: "500+ columns; nothing may wrap it",
        hit: |_, t| t.lines().any(|l| l.len() >= 500),
    },
];

/// `&` / `*` used as an anchor or alias sigil rather than inside a scalar.
fn contains_token(text: &str, sigil: char) -> bool {
    text.lines().any(|line| {
        line.split_whitespace()
            .any(|w| w.starts_with(sigil) && w.len() > 1)
    })
}

struct Finding {
    files: usize,
    why: &'static str,
}

/// Survey every `.yaml` file under `corpora/`.
///
/// # Errors
///
/// Returns an error if the corpus is absent. A survey of nothing would report
/// that no construct needs covering, which is the most dangerous possible
/// answer — so it is a failure, not an empty report.
pub(crate) fn run(root: &Path) -> Result<String, String> {
    let mut files: Vec<PathBuf> = Vec::new();
    collect(&root.join("corpora"), &mut files);
    files.sort();

    if files.is_empty() {
        return Err(format!(
            "no .yaml files under {}/corpora — run corpora/fetch.sh first.\n\
             A survey with no input would claim no construct needs covering.",
            root.display()
        ));
    }

    let mut findings: BTreeMap<&'static str, Finding> = BTreeMap::new();
    for probe in PROBES {
        findings.insert(
            probe.name,
            Finding {
                files: 0,
                why: probe.why,
            },
        );
    }

    let mut total_bytes = 0u64;
    let mut scanned = 0usize;
    for path in &files {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        scanned += 1;
        total_bytes += u64::try_from(bytes.len()).unwrap_or(u64::MAX);
        // Lossy only for the text-shaped probes; the byte-shaped probes see the
        // real bytes, which is what matters for BOM, CRLF and invalid UTF-8.
        let text = String::from_utf8_lossy(&bytes);
        for probe in PROBES {
            if (probe.hit)(&bytes, &text)
                && let Some(found) = findings.get_mut(probe.name)
            {
                found.files += 1;
            }
        }
    }

    let mut out = String::new();
    let _ = writeln!(out, "corpus survey — YAML constructs");
    let _ = writeln!(out, "{scanned} files, {total_bytes} bytes\n");
    let _ = writeln!(
        out,
        "  {:<22} {:>6} {:>7}  why the K1 oracle must cover it",
        "construct", "files", "share"
    );
    let _ = writeln!(out, "  {}", "-".repeat(100));

    // Sorted by descending frequency, then by name so ties are stable.
    let mut rows: Vec<(&str, &Finding)> = findings.iter().map(|(k, v)| (*k, v)).collect();
    rows.sort_by(|a, b| b.1.files.cmp(&a.1.files).then_with(|| a.0.cmp(b.0)));
    for (name, found) in rows {
        // Percentages of a file count that never exceeds a few thousand; the
        // f64 mantissa is not in danger, and a share is a display value anyway.
        #[allow(clippy::cast_precision_loss)]
        let share = if scanned == 0 {
            0.0
        } else {
            100.0 * found.files as f64 / scanned as f64
        };
        let _ = writeln!(
            out,
            "  {name:<22} {:>6} {share:>6.1}%  {}",
            found.files, found.why
        );
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
        } else if path.extension().is_some_and(|e| e == "yaml") {
            out.push(path);
        }
    }
}
