//! Generate the published conformance report — konflux **P4**.
//!
//! MASTER_PLAN §4.1 P4: conformance pass rates published *"including honest
//! failure lists"*. Measuring them was already done by `gate/conformance`; this
//! is the half that makes them *published* rather than printed into a CI log
//! that expires, with the list complete rather than truncated at fifteen.
//!
//! Like `corpus-k1`, this lives in `xtask` rather than in a `#[test]` because
//! the suites are fetched, not committed (ADR-004), and because a test that
//! writes a file into the repository is a blessing mode — `core-verify`
//! invariant V1 says golden files are read, never written. The generator is a
//! command a human runs; the gate that checks its output is the test.

use std::path::Path;

use core_formats::{Format, Json, Yaml};
use core_verify::conformance::{Verdict, publish_report};

/// Where the published report lives, relative to the workspace root.
const PUBLISHED: &str = "conformance/REPORT.md";

/// Render the report, and optionally write it to `conformance/REPORT.md`.
///
/// # Errors
///
/// Errors when a suite is not fetched, when a case cannot be read, or when the
/// file cannot be written.
pub(crate) fn run(root: &Path, write: bool) -> Result<String, String> {
    let text = publish_report(&root.join("conformance"), verdict(&Json), verdict(&Yaml))?;

    if !write {
        return Ok(text);
    }

    let path = root.join(PUBLISHED);
    std::fs::write(&path, text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(format!(
        "wrote {PUBLISHED} ({} bytes)\n\
         \n\
         Review the diff before committing. gate/conformance regenerates this file\n\
         and byte-compares it, so what lands here is what the parser actually does.\n",
        text.len()
    ))
}

/// The conformance verdict adapter: a tree means accepted, an error means
/// refused. Exactly the same function the gate uses, because a report measured
/// by a second definition of "accepted" would publish a different number.
fn verdict<F: Format>(format: &F) -> impl Fn(&[u8]) -> Verdict + '_ {
    move |bytes: &[u8]| {
        if format.parse(bytes).is_ok() {
            Verdict::Accepted
        } else {
            Verdict::Rejected
        }
    }
}
