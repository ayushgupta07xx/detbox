//! The CLI contract — `core-cli` **C2** and **C3**.
//!
//! Exit codes are a public interface. A CI job and, at M4, a git merge driver
//! branch on them without reading stdout, so each one gets a test that runs the
//! real binary. A unit test on an enum would prove the numbers exist; only this
//! proves the binary returns them.

// Entirely test code; clippy's allow-*-in-tests covers only `#[test]` fns.
#![allow(clippy::expect_used, clippy::panic)]

use std::path::PathBuf;
use std::process::{Command, Output};

/// Write two documents into a private directory and diff them.
///
/// Each case gets its own directory named for the case, so a failure can be
/// reproduced by hand and two cases can never observe each other's files.
fn diff(case: &str, extension: &str, a: &str, b: &str, flags: &[&str]) -> Output {
    let dir = std::env::temp_dir().join(format!("konflux-cli-{case}"));
    std::fs::create_dir_all(&dir).expect("create case dir");
    let left = dir.join(format!("a.{extension}"));
    let right = dir.join(format!("b.{extension}"));
    std::fs::write(&left, a).expect("write a");
    std::fs::write(&right, b).expect("write b");

    let mut command = Command::new(env!("CARGO_BIN_EXE_konflux"));
    command.arg("diff");
    for flag in flags {
        command.arg(flag);
    }
    command.arg(&left).arg(&right);
    // Cleared rather than set: the point is that piped output has no colour
    // even when nothing asked for that, so the test must not be the reason.
    command.env_remove("NO_COLOR");
    command.output().expect("konflux runs")
}

fn code(output: &Output) -> i32 {
    output.status.code().unwrap_or(-1)
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn a_semantic_change_exits_one() {
    let out = diff("semantic", "yaml", "replicas: 2\n", "replicas: 3\n", &[]);
    assert_eq!(code(&out), 1, "stdout: {}", stdout(&out));
    assert!(stdout(&out).contains("/replicas"));
}

#[test]
fn a_formatting_only_change_exits_zero() {
    // The product thesis as a number. These two files differ in bytes and not
    // in meaning, and a CI job asking "did anything change?" must not be woken
    // up for it. If this ever returns 1, konflux has become a line differ.
    let out = diff("formatting", "yaml", "a: 1\nb: 2\n", "b: 2\na: 1\n", &[]);
    assert_eq!(code(&out), 0, "stdout: {}", stdout(&out));
    assert!(
        stdout(&out).contains("formatting"),
        "the change is still reported, just not as a finding: {}",
        stdout(&out)
    );
}

#[test]
fn identical_files_exit_zero_and_say_so() {
    let out = diff("identical", "yaml", "a: 1\n", "a: 1\n", &[]);
    assert_eq!(code(&out), 0);
    assert!(stdout(&out).contains("no changes"));
}

#[test]
fn a_refusal_exits_three_and_is_not_confused_with_a_finding() {
    // ADR-012 and the reason `Unmodelled` is its own code: at M4 a merge driver
    // must tell "these differ, resolve them" from "I cannot read this, hand it
    // back to git". Exit 1 and exit 3 demand opposite responses.
    let out = diff("refusal", "yaml", "a: &x 1\n", "a: &y 1\n", &[]);
    assert_eq!(code(&out), 3, "stdout: {}", stdout(&out));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("refused"), "{stderr}");
    assert!(
        stdout(&out).is_empty(),
        "a refusal must not print a diff: {}",
        stdout(&out)
    );
}

#[test]
fn a_bad_flag_exits_two() {
    let mut command = Command::new(env!("CARGO_BIN_EXE_konflux"));
    let out = command
        .args(["diff", "--nope", "a.yaml", "b.yaml"])
        .output()
        .expect("runs");
    assert_eq!(code(&out), 2);
}

#[test]
fn an_unreadable_path_exits_two_not_zero() {
    // The failure that matters most in a script: a typo'd path must never look
    // like "no changes".
    let mut command = Command::new(env!("CARGO_BIN_EXE_konflux"));
    let missing = std::env::temp_dir().join("konflux-cli-does-not-exist.yaml");
    let out = command
        .arg("diff")
        .arg(&missing)
        .arg(&missing)
        .output()
        .expect("runs");
    assert_eq!(code(&out), 2, "a missing file must not read as clean");
}

#[test]
fn check_mode_prints_nothing_at_all() {
    // C2: `--check` distinguishes clean from found *without* parsing stdout.
    let out = diff(
        "check",
        "yaml",
        "replicas: 2\n",
        "replicas: 3\n",
        &["--check"],
    );
    assert_eq!(code(&out), 1);
    assert!(stdout(&out).is_empty(), "got: {}", stdout(&out));
}

#[test]
fn json_output_is_the_library_contract_verbatim() {
    let out = diff(
        "json",
        "yaml",
        "replicas: 2\n",
        "replicas: 3\n",
        &["--json"],
    );
    assert_eq!(code(&out), 1);
    let text = stdout(&out);
    assert!(text.contains("\"schema_version\": 1"));
    assert!(text.contains("\"path\": \"/replicas\""));
    assert!(text.ends_with("}\n"), "trailing bytes are contract too");
}

#[test]
fn piped_output_carries_no_colour() {
    // C3. `Command::output` is a pipe, never a terminal, so this is the real
    // condition rather than a simulated one. Colour in a pipe corrupts data.
    let out = diff("nocolour", "yaml", "replicas: 2\n", "replicas: 3\n", &[]);
    assert!(
        !stdout(&out).contains('\u{1b}'),
        "ANSI escape reached a pipe: {:?}",
        stdout(&out)
    );
}

#[test]
fn two_runs_produce_identical_bytes() {
    // K3 at the CLI boundary. Determinism proven on the library is worth
    // nothing if the binary reorders or re-renders it.
    let first = diff(
        "determinism",
        "yaml",
        "b: 1\na: 2\n",
        "a: 3\nb: 4\n",
        &["--json"],
    );
    let second = diff(
        "determinism",
        "yaml",
        "b: 1\na: 2\n",
        "a: 3\nb: 4\n",
        &["--json"],
    );
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn diffing_across_formats_is_refused_as_a_usage_error() {
    let dir = std::env::temp_dir().join("konflux-cli-mixed");
    std::fs::create_dir_all(&dir).expect("create dir");
    let yaml = dir.join("a.yaml");
    let json = dir.join("b.json");
    std::fs::write(&yaml, "a: 1\n").expect("write");
    std::fs::write(&json, "{\"a\":1}\n").expect("write");
    let out = Command::new(env!("CARGO_BIN_EXE_konflux"))
        .arg("diff")
        .arg(&yaml)
        .arg(&json)
        .output()
        .expect("runs");
    assert_eq!(code(&out), 2);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("different formats"), "{stderr}");
}

#[test]
fn help_exits_clean_and_documents_every_exit_code() {
    // A contract nobody can read is not a contract.
    let out = Command::new(env!("CARGO_BIN_EXE_konflux"))
        .arg("--help")
        .output()
        .expect("runs");
    assert_eq!(code(&out), 0);
    let text = stdout(&out);
    for code in ["0", "1", "2", "3"] {
        assert!(text.contains(code), "exit code {code} is undocumented");
    }
}

#[test]
fn an_unknown_subcommand_names_what_is_supported() {
    let out = Command::new(env!("CARGO_BIN_EXE_konflux"))
        .args(["merge", "a", "b"])
        .output()
        .expect("runs");
    assert_eq!(code(&out), 2);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("M3"),
        "should say when merge arrives: {stderr}"
    );
}

/// Guard against the suite silently testing a stale binary.
#[test]
fn the_binary_under_test_exists() {
    let path = PathBuf::from(env!("CARGO_BIN_EXE_konflux"));
    assert!(path.is_file(), "no binary at {}", path.display());
}
