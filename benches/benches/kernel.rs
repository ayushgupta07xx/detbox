//! Kernel benchmarks.
//!
//! Phase 0 measures the K1 boundary in its degenerate form, so that the
//! benchmark-regression gate (MASTER_PLAN §8) has something real to measure and
//! `xtask bench-compare` has a name to track. At konflux M1 these are replaced
//! by parse/serialize benchmarks over corpus files.
//!
//! Benchmark names are a contract: `benches/baselines/*.tsv` lists every one of
//! them, and the gate fails if a name appears in one place and not the other.
//! Renaming a benchmark is therefore a reviewed change, not a silent one.

// `criterion_group!`/`criterion_main!` expand to undocumented items. The
// workspace denies `missing_docs`; a macro's expansion is not ours to document.
#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

fn k1_identity(c: &mut Criterion) {
    let one_kib = vec![b'x'; 1024];
    c.bench_function("k1_identity_1kib", |b| {
        b.iter(|| core_cst::roundtrip_identity(black_box(&one_kib)));
    });

    let sixty_four_kib = vec![b'x'; 64 * 1024];
    c.bench_function("k1_identity_64kib", |b| {
        b.iter(|| core_cst::roundtrip_identity(black_box(&sixty_four_kib)));
    });
}

criterion_group!(benches, k1_identity);
criterion_main!(benches);
