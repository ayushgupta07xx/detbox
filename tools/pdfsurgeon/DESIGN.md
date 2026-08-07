# DESIGN — `pdfsurgeon`

## Scope
Lossless PDF page operations and forms. **MVP:** merge/split/reorder/rotate with
incremental save (untouched objects byte-preserved); AcroForm fill → flatten
with correct appearance-stream generation. **Full:** PDF/A via veraPDF-verified
conformance, verified redaction, attachment/metadata surgery.

**Non-goals:** a WYSIWYG editor; content editing; OCR (probabilistic — banned);
out-featuring Stirling-PDF wholesale. We win the narrow lossless/forms lane.

## Architecture
PDF is an object graph plus an xref table, not a text CST. This tool owns its
own object-model crate rather than reusing `core-cst`. It still answers to
`core-verify` — the harness is shared even when the kernel is not.

## Proof obligations (MASTER_PLAN §4.4)
| ID | Statement |
|----|-----------|
| P1 | qpdf structural-equivalence + byte-preservation on untouched pages |
| P2 | Render-diff golden images across ≥2 independent renderers (pdfium + Poppler/MuPDF) within a pixel threshold, per operation, per corpus file |
| P3 | Filled+flattened forms render-diff-identical across the renderer set |
| P4 | veraPDF conformance wherever PDF/A is claimed |
| P5 | Parser fuzzed on a hostile-PDF corpus — never panics, never mis-saves |

The pixel threshold in P2 is a golden threshold: it may be tightened, never
loosened, and never without `[NEEDS-AYUSH-APPROVAL]`.

## Current milestone
**Phase 0 — scaffold.** Phase 4. Order vs `veritas` is **Decision D5**, Ayush's,
decided by user pull from Launches 1–3.
