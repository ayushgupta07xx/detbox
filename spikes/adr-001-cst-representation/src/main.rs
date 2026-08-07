//! ADR-001 evidence: three CST representations, measured on the real corpus.
//!
//! MASTER_PLAN §3.1 says the choice between a green/red tree and an owned token
//! tree is "made after a 2-day spike comparing edit ergonomics and memory
//! footprint." MILESTONES adds: *measured, not argued*. This is the measurement.
//!
//! A third candidate — a flat arena of spans — is included because it is the
//! obvious Rust alternative to both and excluding it would make the comparison
//! a formality.
//!
//! ## Method
//!
//! One shared lossless lexer (`lex.rs`) produces the token stream and the
//! nesting plan, so all three build **structurally identical trees** and the
//! only variable is representation. For every file in the YAML corpus:
//!
//! 1. **K1** — `serialize(build(x)) == x`, byte-identical.
//! 2. **K2** — replace one token, then check every byte outside that token's
//!    span is unchanged, against an independently computed expected output.
//! 3. **Memory** — live heap bytes held by the tree, counted by a global
//!    allocator rather than sampled from RSS, so the number is exactly
//!    reproducible.
//! 4. **Persistent edit** — live bytes needed to hold the pre-edit *and*
//!    post-edit trees at once. This is the number that decides the argument:
//!    a three-way merge and an undo stack both need more than one version.
//!
//! Everything above is deterministic. Timings are reported separately and are
//! explicitly machine-dependent — they are a relative, same-machine sanity
//! check, not a published benchmark (Appendix C bans those unless they carry
//! full methodology).
//!
//! ## Run
//!
//! ```text
//! corpora/fetch.sh                                   # once
//! cargo run --release --manifest-path spikes/adr-001-cst-representation/Cargo.toml
//! ```

mod alloc;
mod arena;
mod green_red;
mod lex;
mod owned;

use std::path::{Path, PathBuf};
use std::time::Instant;

#[global_allocator]
static ALLOC: alloc::Counting = alloc::Counting;

/// Fixed replacement text for the K2 edit. Deliberately a different length from
/// most tokens, so a length-change bug cannot hide.
const EDIT_TEXT: &[u8] = b"SPIKE-EDIT";

#[derive(Default)]
struct Stats {
    files: usize,
    src_bytes: u64,
    tree_bytes: u64,
    allocs: u64,
    nodes: u64,
    tokens: u64,
    k1_pass: usize,
    k2_pass: usize,
    k2_attempted: usize,
    both_versions_bytes: u64,
    edit_allocs: u64,
    inplace_allocs: u64,
    locate_allocs: u64,
    locate_correct: usize,
    locate_attempted: usize,
}

impl Stats {
    fn row(&self, name: &str) -> String {
        format!(
            "  {:<22} {:>10.2}x {:>12} {:>11} {:>10.2}x  {:>5}/{:<5} {:>5}/{:<5}",
            name,
            self.tree_bytes as f64 / self.src_bytes as f64,
            self.tree_bytes,
            self.allocs,
            self.both_versions_bytes as f64 / self.tree_bytes as f64,
            self.k1_pass,
            self.files,
            self.k2_pass,
            self.k2_attempted,
        )
    }
}

fn main() {
    let root = workspace_root();
    let files = corpus_files(&root);
    if files.is_empty() {
        eprintln!(
            "no corpus found under {}/corpora — run corpora/fetch.sh first",
            root.display()
        );
        std::process::exit(1);
    }

    let mut green = Stats::default();
    let mut owned_s = Stats::default();
    let mut arena_s = Stats::default();
    let mut intern_hits = 0u64;
    let mut intern_total = 0u64;

    for path in &files {
        let Ok(src) = std::fs::read(path) else { continue };
        if src.is_empty() {
            continue;
        }
        let lexed = lex::lex(&src);
        // The lexer must be lossless before any tree is judged, or a K1 failure
        // downstream is unattributable.
        assert_eq!(
            lexed.covered(),
            src.len(),
            "lexer dropped bytes in {}",
            path.display()
        );
        let parents = lex::nesting(&lexed.lines);

        // Deterministic edit target: the Word token nearest the middle.
        let word_idxs: Vec<usize> = lexed
            .toks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.kind == lex::Kind::Word)
            .map(|(i, _)| i)
            .collect();
        let target = word_idxs.get(word_idxs.len() / 2).copied();
        if target.is_none() {
            let rel = path.strip_prefix(&root).unwrap_or(path);
            let rel: Vec<&str> = rel.components().filter_map(|c| c.as_os_str().to_str()).collect();
            println!("  SKIPPED (no Word token to edit): {}", rel.join("/"));
        }
        let expected_after_edit = target.map(|n| {
            let tok = lexed.toks[n];
            let mut out = Vec::with_capacity(src.len() + EDIT_TEXT.len());
            out.extend_from_slice(&src[..tok.start as usize]);
            out.extend_from_slice(EDIT_TEXT);
            out.extend_from_slice(&src[(tok.start + tok.len) as usize..]);
            out
        });

        measure_green(
            &src, &lexed, &parents, target, expected_after_edit.as_deref(),
            &mut green, &mut intern_hits, &mut intern_total,
        );
        measure_owned(
            &src, &lexed, &parents, target, expected_after_edit.as_deref(), &mut owned_s,
        );
        measure_arena(
            &src, &lexed, &parents, target, expected_after_edit.as_deref(), &mut arena_s,
        );
    }

    // ---------------- deterministic report ----------------
    println!("ADR-001 spike — CST representation");
    println!("corpus: {} YAML files, {} bytes\n", green.files, green.src_bytes);
    println!(
        "  {:<22} {:>11} {:>12} {:>11} {:>11}  {:>5} {:>11}",
        "representation", "bytes/byte", "tree bytes", "allocs", "v1+v2/v1", "K1", "K2*"
    );
    println!("  {}", "-".repeat(94));
    println!("{}", green.row("A green/red (rowan)"));
    println!("{}", owned_s.row("B owned token tree"));
    println!("{}", arena_s.row("C flat arena of spans"));

    println!("\n  nodes/tokens built (identical by construction):");
    println!("    A {} nodes, {} tokens", green.nodes, green.tokens);
    println!("    B {} nodes, {} tokens", owned_s.nodes, owned_s.tokens);
    println!("    C {} nodes, {} tokens", arena_s.nodes, arena_s.tokens);
    println!(
        "\n  A token interning: {} hits / {} lookups = {:.1}% reuse",
        intern_hits,
        intern_total,
        100.0 * intern_hits as f64 / intern_total as f64
    );
    println!(
        "  allocations per input KiB:  A {:.1}   B {:.1}   C {:.3}",
        green.allocs as f64 / (green.src_bytes as f64 / 1024.0),
        owned_s.allocs as f64 / (owned_s.src_bytes as f64 / 1024.0),
        arena_s.allocs as f64 / (arena_s.src_bytes as f64 / 1024.0),
    );
    println!(
        "\n  one edit, PERSISTENT (pre-edit version stays live) — allocations:"
    );
    println!(
        "    A {:>8}   B {:>8}   C {:>8}",
        green.edit_allocs, owned_s.edit_allocs, arena_s.edit_allocs
    );
    println!("  one edit, DESTRUCTIVE (old version discarded) — allocations:");
    println!(
        "    A {:>8}   B {:>8}   C {:>8}     (A is immutable: no cheaper path exists)",
        green.inplace_allocs, owned_s.inplace_allocs, arena_s.inplace_allocs
    );
    println!("\n  locate(token) -> absolute source range, {} files:", green.files);
    println!(
        "    allocations:  A {:>8}   B {:>8}   C {:>8}",
        green.locate_allocs, owned_s.locate_allocs, arena_s.locate_allocs
    );
    println!(
        "    correct:      A {:>5}/{:<5} B {:>5}/{:<5} C {:>5}/{:<5}",
        green.locate_correct, green.locate_attempted,
        owned_s.locate_correct, owned_s.locate_attempted,
        arena_s.locate_correct, arena_s.locate_attempted,
    );
    println!(
        "\n  * K2 is measured over files where an edit target exists ({} of {} files).",
        green.k2_attempted, green.files
    );
    if green.k2_attempted != green.files {
        println!(
            "    {} file(s) had no Word token to edit and were SKIPPED, not passed.",
            green.files - green.k2_attempted
        );
    }

    // ---------------- machine-dependent report ----------------
    println!("\n  timings — MACHINE-DEPENDENT, relative comparison only.");
    println!("  Not a published benchmark: no methodology page, no fixed runner (Appendix C).");
    let t = timings(&files);
    println!("    build+serialize whole corpus:  A {:?}   B {:?}   C {:?}", t.0, t.1, t.2);
}

fn measure_green(
    src: &[u8],
    lexed: &lex::Lexed,
    parents: &[u32],
    target: Option<usize>,
    expected: Option<&[u8]>,
    stats: &mut Stats,
    intern_hits: &mut u64,
    intern_total: &mut u64,
) {
    let live0 = alloc::live();
    let allocs0 = alloc::allocs();
    let tree = green_red::build(src, lexed, parents);
    let tree_bytes = alloc::live() - live0;
    let allocs = alloc::allocs() - allocs0;

    stats.files += 1;
    stats.src_bytes += src.len() as u64;
    stats.tree_bytes += tree_bytes as u64;
    stats.allocs += allocs as u64;
    let (nodes, tokens) = green_red::count_nodes(&tree);
    stats.nodes += nodes as u64;
    stats.tokens += tokens as u64;
    *intern_hits += tree.intern_hits as u64;
    *intern_total += (tree.intern_hits + tree.intern_misses) as u64;

    if green_red::serialize(&tree) == src {
        stats.k1_pass += 1;
    }

    if let (Some(n), Some(expected)) = (target, expected) {
        if let Some(path) = green_red::nth_token_path(&tree, n) {
            stats.k2_attempted += 1;
            stats.locate_attempted += 1;
            let want = expected_range(lexed, n);
            let a0 = alloc::allocs();
            let got = green_red::locate(&tree.root, &path);
            stats.locate_allocs += (alloc::allocs() - a0) as u64;
            if got == want {
                stats.locate_correct += 1;
            }

            let before_live = alloc::live();
            let before_allocs = alloc::allocs();
            // Persistent by construction: `tree.root` stays valid and shared.
            let v2 = green_red::replace_token(&tree.root, &path, EDIT_TEXT);
            stats.both_versions_bytes += (tree_bytes + (alloc::live() - before_live)) as u64;
            let cost = (alloc::allocs() - before_allocs) as u64;
            stats.edit_allocs += cost;
            // The green tree is immutable: there is no cheaper destructive path.
            stats.inplace_allocs += cost;
            let v2_tree = green_red::Tree { root: v2, intern_hits: 0, intern_misses: 0 };
            if green_red::serialize(&v2_tree) == expected {
                stats.k2_pass += 1;
            }
        }
    }
}

/// The range the edited token must occupy, computed straight from the lexer —
/// an oracle independent of every tree under test.
fn expected_range(lexed: &lex::Lexed, n: usize) -> (u32, u32) {
    let tok = lexed.toks[n];
    (tok.start, tok.start + tok.len)
}

fn measure_owned(
    src: &[u8],
    lexed: &lex::Lexed,
    parents: &[u32],
    target: Option<usize>,
    expected: Option<&[u8]>,
    stats: &mut Stats,
) {
    let live0 = alloc::live();
    let allocs0 = alloc::allocs();
    let mut tree = owned::build(src, lexed, parents);
    let tree_bytes = alloc::live() - live0;

    stats.files += 1;
    stats.src_bytes += src.len() as u64;
    stats.tree_bytes += tree_bytes as u64;
    stats.allocs += (alloc::allocs() - allocs0) as u64;
    let (nodes, tokens) = owned::count_nodes(&tree);
    stats.nodes += nodes as u64;
    stats.tokens += tokens as u64;

    if owned::serialize(&tree) == src {
        stats.k1_pass += 1;
    }

    if let (Some(n), Some(expected)) = (target, expected) {
        if let Some(path) = owned::nth_token_path(&tree, n) {
            stats.k2_attempted += 1;
            stats.locate_attempted += 1;
            let want = expected_range(lexed, n);
            let a0 = alloc::allocs();
            let got = owned::locate(&tree, &path);
            stats.locate_allocs += (alloc::allocs() - a0) as u64;
            if got == want {
                stats.locate_correct += 1;
            }

            // Persistent: keeping the pre-edit version means duplicating the
            // whole tree — there is nothing to share. Rebuilding from the same
            // inputs is the cheapest honest way to get a second live copy.
            let before_live = alloc::live();
            let before_allocs = alloc::allocs();
            let mut v2 = owned::build(src, lexed, parents);
            owned::replace_token(&mut v2, &path, EDIT_TEXT);
            stats.both_versions_bytes += (tree_bytes + (alloc::live() - before_live)) as u64;
            stats.edit_allocs += (alloc::allocs() - before_allocs) as u64;
            if owned::serialize(&v2) == expected {
                stats.k2_pass += 1;
            }
            drop(v2);

            // Destructive: mutate v1 in place. This is where B shines, and it
            // is only available when nobody needs the old version.
            let a0 = alloc::allocs();
            owned::replace_token(&mut tree, &path, EDIT_TEXT);
            stats.inplace_allocs += (alloc::allocs() - a0) as u64;
        }
    }
}

fn measure_arena(
    src: &[u8],
    lexed: &lex::Lexed,
    parents: &[u32],
    target: Option<usize>,
    expected: Option<&[u8]>,
    stats: &mut Stats,
) {
    let live0 = alloc::live();
    let allocs0 = alloc::allocs();
    let tree = arena::build(src, lexed, parents);
    let tree_bytes = alloc::live() - live0;

    stats.files += 1;
    stats.src_bytes += src.len() as u64;
    stats.tree_bytes += tree_bytes as u64;
    stats.allocs += (alloc::allocs() - allocs0) as u64;
    let (nodes, tokens) = arena::count_nodes(&tree);
    stats.nodes += nodes as u64;
    stats.tokens += tokens as u64;

    if arena::serialize(&tree) == src {
        stats.k1_pass += 1;
    }

    if let (Some(n), Some(expected)) = (target, expected) {
        if let Some(id) = arena::nth_token(&tree, n) {
            stats.k2_attempted += 1;
            stats.locate_attempted += 1;
            let want = expected_range(lexed, n);
            let a0 = alloc::allocs();
            let got = arena::locate(&tree, id);
            stats.locate_allocs += (alloc::allocs() - a0) as u64;
            if got == want {
                stats.locate_correct += 1;
            }

            let before_live = alloc::live();
            let before_allocs = alloc::allocs();
            // Persistence means cloning the arena: elements, overrides and the
            // source buffer. Nothing is shared.
            let mut v2 = arena::Tree {
                elems: tree.elems.clone(),
                overrides: tree.overrides.clone(),
                root: tree.root,
                src: tree.src.clone(),
            };
            arena::replace_token(&mut v2, id, EDIT_TEXT);
            stats.both_versions_bytes += (tree_bytes + (alloc::live() - before_live)) as u64;
            stats.edit_allocs += (alloc::allocs() - before_allocs) as u64;
            if arena::serialize(&v2) == expected {
                stats.k2_pass += 1;
            }
            drop(v2);

            // Destructive: one push into the override table, one field write.
            let mut v1 = tree;
            let a0 = alloc::allocs();
            arena::replace_token(&mut v1, id, EDIT_TEXT);
            stats.inplace_allocs += (alloc::allocs() - a0) as u64;
        }
    }
}

type Timings = (std::time::Duration, std::time::Duration, std::time::Duration);

fn timings(files: &[PathBuf]) -> Timings {
    let sources: Vec<Vec<u8>> = files
        .iter()
        .filter_map(|p| std::fs::read(p).ok())
        .filter(|s| !s.is_empty())
        .collect();

    let mut out = [std::time::Duration::ZERO; 3];
    for (slot, which) in out.iter_mut().enumerate() {
        let start = Instant::now();
        for src in &sources {
            let lexed = lex::lex(src);
            let parents = lex::nesting(&lexed.lines);
            let bytes = match slot {
                0 => green_red::serialize(&green_red::build(src, &lexed, &parents)),
                1 => owned::serialize(&owned::build(src, &lexed, &parents)),
                _ => arena::serialize(&arena::build(src, &lexed, &parents)),
            };
            std::hint::black_box(bytes);
        }
        *which = start.elapsed();
    }
    (out[0], out[1], out[2])
}

fn workspace_root() -> PathBuf {
    // spikes/adr-001-cst-representation -> spikes -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Every `.yaml` file in the fetched corpus, byte-wise sorted so two runs see
/// the same files in the same order.
fn corpus_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.join("corpora")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "yaml") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}
