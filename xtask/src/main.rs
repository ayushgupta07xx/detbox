//! `xtask` — repo automation (MASTER_PLAN §2).
//!
//! Everything CI does that is not `cargo <verb>` runs through here, so that a
//! contributor can reproduce a CI failure with the same command CI ran, on any
//! OS, with no shell dialect in between. No third-party dependencies: the tool
//! that verifies determinism should not itself be a supply chain.
//!
//! ```text
//! cargo xtask scaffold-report          deterministic JSON over the repo tree
//! cargo xtask hash <file>              SHA-256 of a file, lowercase hex
//! cargo xtask assert-equal <a> <b>     byte-compare two files; exit 1 if they differ
//! cargo xtask corpus-verify            offline validation of corpora/sources/*.sources
//! cargo xtask corpus-survey            which YAML constructs the corpus contains
//! cargo xtask corpus-k1                K1 over every corpus file (konflux P1)
//! cargo xtask bench-compare <dir> <baseline>   criterion output vs saved baseline
//! ```

mod bench;
mod corpus;
mod k1;
mod report;
mod sha256;
mod survey;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some((command, rest)) = args.split_first() else {
        usage();
        // Exit code 2 = usage error (core-cli C2).
        return ExitCode::from(2);
    };

    let result = match command.as_str() {
        "scaffold-report" => cmd_scaffold_report(),
        "hash" => cmd_hash(rest),
        "assert-equal" => cmd_assert_equal(rest),
        "corpus-verify" => cmd_corpus_verify(),
        "corpus-survey" => cmd_corpus_survey(),
        "corpus-k1" => k1::run(&workspace_root()),
        "bench-compare" => cmd_bench_compare(rest),
        "-h" | "--help" | "help" => {
            usage();
            return ExitCode::SUCCESS;
        }
        other => Err(format!("unknown command `{other}`")),
    };

    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("xtask: {message}");
            // Exit code 1 = "a finding the caller asked about" (core-cli C2).
            ExitCode::from(1)
        }
    }
}

fn usage() {
    eprintln!(
        "usage: cargo xtask <command>\n\
         \n\
         \x20 scaffold-report                     deterministic JSON over the repo tree\n\
         \x20 hash <file>                         SHA-256 of a file, lowercase hex\n\
         \x20 assert-equal <a> <b>                byte-compare two files\n\
         \x20 corpus-verify                       validate corpora/sources/*.sources (offline)\n\
         \x20 corpus-survey                       which YAML constructs the corpus contains\n\
         \x20 corpus-k1                           K1 over every corpus file (konflux P1)\n\
         \x20 bench-compare <dir> <baseline>      criterion output vs saved baseline"
    );
}

/// The workspace root: `xtask/`'s parent.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

fn cmd_scaffold_report() -> Result<String, String> {
    report::render(&workspace_root()).map_err(|e| format!("scaffold-report failed: {e}"))
}

fn cmd_hash(args: &[String]) -> Result<String, String> {
    let Some(path) = args.first() else {
        return Err("hash: expected a file path".into());
    };
    let bytes = std::fs::read(path).map_err(|e| format!("hash: cannot read {path}: {e}"))?;
    Ok(format!("{}\n", sha256::hex(&bytes)))
}

fn cmd_assert_equal(args: &[String]) -> Result<String, String> {
    let (Some(a), Some(b)) = (args.first(), args.get(1)) else {
        return Err("assert-equal: expected two file paths".into());
    };
    let left = std::fs::read(a).map_err(|e| format!("assert-equal: cannot read {a}: {e}"))?;
    let right = std::fs::read(b).map_err(|e| format!("assert-equal: cannot read {b}: {e}"))?;
    if left == right {
        return Ok(format!(
            "identical: {a} == {b} ({} bytes, sha256 {})\n",
            left.len(),
            sha256::hex(&left)
        ));
    }
    let at = left
        .iter()
        .zip(right.iter())
        .position(|(l, r)| l != r)
        .unwrap_or_else(|| left.len().min(right.len()));
    Err(format!(
        "DETERMINISM VIOLATION\n\
         \x20 {a}: {} bytes, sha256 {}\n\
         \x20 {b}: {} bytes, sha256 {}\n\
         \x20 first difference at byte {at}\n\
         \n\
         Two runs of the same code on the same input produced different bytes.\n\
         This is invariant K3 (MASTER_PLAN §3.1) and it is not negotiable.\n\
         \n\
         In the code: HashMap iteration in an output path, a wall-clock read, an\n\
         unstable sort, a platform path separator, or locale-dependent formatting\n\
         (MASTER_PLAN §9.5).\n\
         \n\
         In the harness: check the two runs actually had the same input. Writing\n\
         run one's output into the directory run two reads will fail here and\n\
         prove nothing about the code.",
        left.len(),
        sha256::hex(&left),
        right.len(),
        sha256::hex(&right)
    ))
}

fn cmd_corpus_verify() -> Result<String, String> {
    corpus::verify(&workspace_root().join("corpora/sources"))
        .map_err(|problems| format!("corpus manifests are invalid:\n{problems}"))
}

fn cmd_corpus_survey() -> Result<String, String> {
    survey::run(&workspace_root())
}

fn cmd_bench_compare(args: &[String]) -> Result<String, String> {
    let (Some(dir), Some(baseline)) = (args.first(), args.get(1)) else {
        return Err("bench-compare: expected <criterion-dir> <baseline-file>".into());
    };
    bench::compare(Path::new(dir), Path::new(baseline))
}
