//! # `core-cli` — one feel across every tool
//!
//! **Mission (MASTER_PLAN §3.4).** The conventions every binary in this
//! ecosystem obeys, implemented once. A user who learns one tool has learned
//! the argument surface of all nine.
//!
//! ## The law every tool obeys
//!
//! - **`--json`** — stable, machine-readable output. The schema is *versioned*;
//!   a field is never removed or repurposed without a version bump and a
//!   migration note. Breaking it is a public API change (§9.3: human review).
//! - **`--check`** — exit-code-only mode for CI. No stdout noise, no colour,
//!   meaningful exit codes.
//! - **Deterministic output ordering** — always. Two runs on the same input
//!   produce the same bytes, in the same order, on every platform.
//! - **`NO_COLOR` respected** — and colour is off whenever stdout is not a TTY.
//! - **No network access, ever** — unless the tool has an explicit `--online`
//!   flag *and* its README explains why. Offline by default is a brand promise
//!   (§10), not a default setting. Telemetry and phone-home are permanently
//!   banned (Appendix C).
//! - **Span-rich diagnostics** — miette-style: show the bytes, point at the
//!   problem, suggest the fix. A byte offset with no context is not a
//!   diagnostic.
//!
//! ## Invariants
//!
//! - **C1 `--json` is append-only within a schema version.** Enforced by a
//!   golden suite over serialised output.
//! - **C2 Exit codes are a contract.** `0` success, `1` a finding the user
//!   asked about (conflict, divergence, lossy conversion), `2` usage error,
//!   `>2` internal failure. `--check` distinguishes "clean" from "found
//!   something" without parsing stdout.
//! - **C3 Nothing here reads the clock, the locale, the environment's random
//!   state, or the network.** Determinism is a property of the shared layer, so
//!   no tool has to re-earn it.
//! - **C4 Errors carry spans, not strings.** Diagnostics are structured values
//!   that render to human, `--json`, and markdown.
//!
//! ## Status
//!
//! **konflux M2.** Exit-code policy, the shared flag surface, and colour
//! discipline. The diagnostic type (C4) lands with M2's error reporting.
//!
//! ## Why there is no argument-parsing dependency
//!
//! Hand-rolled, and deliberately. The argument surface across this ecosystem is
//! small on purpose (§4.2 calls strukt's query language "small and boring on
//! purpose", and the same restraint applies here), so a parser costs less than
//! the dependency tree it would drag into every published crate. `cargo-deny`
//! has less to audit and `core-cli` stays a leaf. If shell completions later
//! need a generator, that is the moment to revisit — not before. ADR-016.

use std::io::IsTerminal as _;

/// Exit codes, which are a contract and not a convenience (**C2**).
///
/// A caller — a CI job, a git merge driver, a shell script — must be able to
/// tell these apart *without parsing stdout*. That is the whole reason `--check`
/// exists, and the reason [`Self::Unmodelled`] is separate from
/// [`Self::Finding`]: "I found differences" and "I could not read this" demand
/// opposite responses from a merge driver, and collapsing them would make it
/// take one side of a file it never understood (ADR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Exit {
    /// Nothing to report. The question was asked and the answer is "no".
    Clean = 0,
    /// A finding the caller asked about: a difference, a conflict, a loss.
    Finding = 1,
    /// The caller invoked us wrongly. Bad flag, missing argument, unreadable
    /// path.
    Usage = 2,
    /// We refuse: the input is beyond what this tool models. Not a failure of
    /// the input and not a bug — a boundary, reported honestly so the caller
    /// can fall back to something that does handle it.
    Unmodelled = 3,
}

impl Exit {
    /// The process exit code.
    #[must_use]
    pub fn code(self) -> u8 {
        self as u8
    }

    /// As a [`std::process::ExitCode`], for returning from `main`.
    #[must_use]
    pub fn process(self) -> std::process::ExitCode {
        std::process::ExitCode::from(self.code())
    }
}

/// The flags every tool in this ecosystem answers to (§3.4).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Common {
    /// Emit stable, schema-versioned machine output instead of a human view.
    pub json: bool,
    /// Exit-code-only mode: no stdout, no colour.
    pub check: bool,
    /// Print usage and stop.
    pub help: bool,
    /// Positional arguments, in order, with flags removed.
    pub positional: Vec<String>,
}

/// Why an argument list could not be understood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError {
    /// What went wrong, in a sentence a user can act on.
    pub message: String,
}

impl std::fmt::Display for UsageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UsageError {}

impl Common {
    /// Parse the shared flags out of an argument list.
    ///
    /// Everything after a bare `--` is positional, so a path may be named
    /// `--json` without ceremony.
    ///
    /// # Errors
    ///
    /// Returns [`UsageError`] for an unrecognised `-`-prefixed argument. An
    /// unknown flag is a usage error rather than a positional: silently
    /// treating `--jsonn` as a filename is how a script ends up diffing a file
    /// that does not exist and reporting success.
    pub fn parse<I, S>(args: I) -> Result<Self, UsageError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut out = Self::default();
        let mut only_positional = false;
        for arg in args {
            let arg: String = arg.into();
            if only_positional {
                out.positional.push(arg);
                continue;
            }
            match arg.as_str() {
                "--" => only_positional = true,
                "--json" => out.json = true,
                "--check" => out.check = true,
                "-h" | "--help" => out.help = true,
                other if other.starts_with('-') && other != "-" => {
                    return Err(UsageError {
                        message: format!(
                            "unknown flag `{other}`. Pass `--` before a path that starts with a dash."
                        ),
                    });
                }
                _ => out.positional.push(arg),
            }
        }
        Ok(out)
    }
}

/// Whether output may carry ANSI colour.
///
/// Three ways to say no and they all win: `NO_COLOR` set to anything (the
/// [no-color.org](https://no-color.org) convention), `--check` mode, or stdout
/// not being a terminal. Colour in a pipe is corruption of the data, not
/// decoration of it.
///
/// Reading the environment is deliberate and is not a C3 violation: C3 forbids
/// the clock, the locale and unseeded randomness — inputs that vary run to run
/// on the *same* machine and configuration. `NO_COLOR` is configuration, and a
/// user who sets it expects it to be read.
#[must_use]
pub fn colour_allowed(check: bool) -> bool {
    if check || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    std::io::stdout().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::{Common, Exit, colour_allowed};

    #[test]
    fn exit_codes_are_the_documented_contract() {
        // C2. These numbers are a public interface: a merge driver and a CI job
        // branch on them, so changing one is a breaking change.
        assert_eq!(Exit::Clean.code(), 0);
        assert_eq!(Exit::Finding.code(), 1);
        assert_eq!(Exit::Usage.code(), 2);
        assert_eq!(Exit::Unmodelled.code(), 3);
    }

    #[test]
    fn refusing_is_not_the_same_code_as_finding_something() {
        // The distinction M4's merge driver depends on: "these differ" means
        // resolve them, "I cannot read this" means hand back to git.
        assert_ne!(Exit::Finding.code(), Exit::Unmodelled.code());
    }

    #[test]
    fn flags_are_recognised_and_positionals_keep_their_order() {
        let parsed = Common::parse(["--json", "a.yaml", "--check", "b.yaml"]).expect("parses");
        assert!(parsed.json && parsed.check);
        assert_eq!(parsed.positional, ["a.yaml", "b.yaml"]);
    }

    #[test]
    fn a_double_dash_ends_flag_parsing() {
        let parsed = Common::parse(["--json", "--", "--check"]).expect("parses");
        assert!(parsed.json, "flags before -- still apply");
        assert!(!parsed.check, "after --, it is a path");
        assert_eq!(parsed.positional, ["--check"]);
    }

    #[test]
    fn an_unknown_flag_is_a_usage_error_not_a_filename() {
        // Treating `--jsonn` as a path is how a script diffs a file that does
        // not exist and reports success.
        let err = Common::parse(["--jsonn"]).expect_err("unknown flag");
        assert!(err.message.contains("--jsonn"), "{err}");
    }

    #[test]
    fn a_bare_dash_is_a_positional() {
        let parsed = Common::parse(["-"]).expect("parses");
        assert_eq!(parsed.positional, ["-"]);
    }

    #[test]
    fn check_mode_never_permits_colour() {
        // Independent of TTY and of NO_COLOR: --check promises exit-code-only.
        assert!(!colour_allowed(true));
    }
}
