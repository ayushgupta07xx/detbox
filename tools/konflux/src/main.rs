//! `konflux` — structural diff for configs.
//!
//! The first runnable binary in this ecosystem. Everything before it was a
//! library with a test suite, which is enough to prove correctness and not
//! enough to use.
//!
//! Obeys `core-cli` §3.4 in full: `--json`, `--check`, `NO_COLOR`, deterministic
//! ordering, no network. The exit code is the part worth reading twice — see
//! [`core_cli::Exit`].

// A binary's `main` is allowed to be the place errors stop.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::Path;
use std::process::ExitCode;

use core_cli::{Common, Exit};
use core_formats::{Format, Json, Yaml};
use konflux::{ChangeKind, DiffError, DiffReport, Significance};

const USAGE: &str = "\
konflux — structural diff for configs

USAGE:
    konflux diff [OPTIONS] <A> <B>

OPTIONS:
    --json     stable, schema-versioned machine output
    --check    exit code only, no stdout
    -h, --help print this and stop
    --         everything after this is a path

EXIT CODES:
    0  no semantic change
    1  semantic changes found
    2  usage error
    3  refused — konflux cannot model this input

The exit code tracks MEANING, not bytes. Two files that differ only in key
order or quoting exit 0, because nothing about them changed. That is the
distinction this tool exists to make.
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(exit) => exit.process(),
        Err(message) => {
            eprintln!("konflux: {message}");
            eprintln!("\n{USAGE}");
            Exit::Usage.process()
        }
    }
}

fn run(args: &[String]) -> Result<Exit, String> {
    let common = Common::parse(args.iter().cloned()).map_err(|e| e.to_string())?;
    if common.help || common.positional.is_empty() {
        print!("{USAGE}");
        return Ok(Exit::Clean);
    }

    let (command, rest) = common
        .positional
        .split_first()
        .ok_or_else(|| "no subcommand".to_string())?;
    if command != "diff" {
        return Err(format!(
            "unknown subcommand `{command}`. konflux speaks `diff` today; \
             merge arrives at M3."
        ));
    }

    let (Some(a), Some(b)) = (rest.first(), rest.get(1)) else {
        return Err("diff needs two paths".to_string());
    };
    if rest.len() > 2 {
        return Err(format!(
            "diff takes exactly two paths, got {}. konflux compares two \
             documents; a three-way merge is M3.",
            rest.len()
        ));
    }

    let report = diff_paths(Path::new(a), Path::new(b))?;
    Ok(emit(&report, &common))
}

/// Read both sides and diff them through the format their extension names.
fn diff_paths(a: &Path, b: &Path) -> Result<Result<DiffReport, DiffError>, String> {
    let extension = |path: &Path| {
        path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
    };
    let (ext_a, ext_b) = (extension(a), extension(b));
    if ext_a != ext_b {
        return Err(format!(
            "`{}` and `{}` are different formats. A diff across formats is a \
             conversion, which is veritas's job, not konflux's.",
            a.display(),
            b.display()
        ));
    }

    let read = |path: &Path| {
        std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let (left, right) = (read(a)?, read(b)?);

    match ext_a.as_str() {
        "yaml" | "yml" => Ok(konflux::diff(&Yaml, &left, &right)),
        "json" => Ok(konflux::diff(&Json, &left, &right)),
        other => Err(format!(
            "konflux does not read `.{other}` yet. It speaks {} and {} today; \
             toml and hcl arrive in Phase 2.",
            Yaml.name(),
            Json.name()
        )),
    }
}

/// Render the report and decide the exit code.
fn emit(report: &Result<DiffReport, DiffError>, common: &Common) -> Exit {
    let report = match report {
        Ok(report) => report,
        Err(refusal) => {
            // A refusal goes to stderr even under --check: the exit code says
            // "I could not answer", and a caller deserves to know why without
            // having to re-run without the flag.
            eprintln!("{refusal}");
            return Exit::Unmodelled;
        }
    };

    if common.json {
        print!("{}", report.to_json());
    } else if !common.check {
        print!("{}", human(report, core_cli::colour_allowed(common.check)));
    }

    // The exit code tracks meaning, not bytes. A file whose keys were reordered
    // has not changed, and a CI job asking "did anything change?" should not be
    // woken up for it — that is the entire product thesis expressed as a number.
    if report
        .changes
        .iter()
        .any(|change| change.significance == Significance::Semantic)
    {
        Exit::Finding
    } else {
        Exit::Clean
    }
}

/// The human view: one line per change, aligned, ordered as the report is.
fn human(report: &DiffReport, colour: bool) -> String {
    use std::fmt::Write as _;

    if report.changes.is_empty() {
        return "no changes\n".to_string();
    }

    let width = report
        .changes
        .iter()
        .map(|change| display_path(&change.path).chars().count())
        .max()
        .unwrap_or(0);

    let mut out = String::new();
    let mut semantic = 0usize;
    for change in &report.changes {
        let mark = match change.kind {
            ChangeKind::Added => "+",
            ChangeKind::Removed => "-",
            ChangeKind::Changed => "~",
            ChangeKind::Moved => ">",
        };
        let (open, close) = match (colour, change.significance) {
            (false, _) => ("", ""),
            (true, Significance::Semantic) => ("\u{1b}[1m", "\u{1b}[0m"),
            (true, Significance::Formatting) => ("\u{1b}[2m", "\u{1b}[0m"),
        };
        if change.significance == Significance::Semantic {
            semantic += 1;
        }
        let path = display_path(&change.path);
        let _ = write!(out, "{open}{mark} {path:<width$}{close}");
        match (&change.before, &change.after) {
            (Some(before), Some(after)) => {
                let _ = write!(out, "  {}  ->  {}", one_line(before), one_line(after));
            }
            (Some(before), None) => {
                let _ = write!(out, "  {}", one_line(before));
            }
            (None, Some(after)) => {
                let _ = write!(out, "  {}", one_line(after));
            }
            (None, None) => {
                let _ = write!(out, "  (reordered)");
            }
        }
        if change.significance == Significance::Formatting {
            let _ = write!(out, "  [formatting]");
        }
        out.push('\n');
    }

    let formatting = report.changes.len() - semantic;
    let _ = writeln!(out, "\n{semantic} semantic, {formatting} formatting");
    out
}

/// `""` is the document root, which prints as nothing at all otherwise.
fn display_path(path: &str) -> String {
    if path.is_empty() {
        "(root)".to_string()
    } else {
        path.to_string()
    }
}

/// Collapse a value to one line so the table stays a table. Block scalars and
/// multi-line templates are common and would otherwise shred the layout.
fn one_line(value: &str) -> String {
    let flat = value.replace('\n', "\\n");
    if flat.chars().count() <= 40 {
        return flat;
    }
    let head: String = flat.chars().take(37).collect();
    format!("{head}...")
}
