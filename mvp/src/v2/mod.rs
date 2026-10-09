//! AIL Runtime **v2** — audit-branch variant of the semantic layer.
//!
//! Everything in this module is additive: the v1 modules (`connectivity`,
//! `vizgraph`, `lever`, the interpreter in `main.rs`) are left untouched and
//! continue to compile. v2 is a parallel, self-contained implementation of the
//! same ideas with the defects found during the audit removed:
//!
//! | v1 problem                                             | v2 answer                                   |
//! |--------------------------------------------------------|---------------------------------------------|
//! | relation graph declared by hand (`link_functions`)     | [`derive`] computes it from the Binary AST   |
//! | reinforcing an edge *lowers* its saturation            | [`relation`] monotone three-component model  |
//! | D3 unreachable below K6, "✕ ⇒ D3" never holds         | symbol prior + weighted Jaccard topology     |
//! | weakening check = "function name still exists"         | [`contract`] symbolic effect/guard refinement|
//! | ownership analyzer exists only as a document           | [`ownership`] static analyzer + VM alias check|
//! | lineage hash ignores parent ⇒ identical ids            | [`lineage`] Merkle chain ids                 |
//! | GERMES antipode table partly wrong                     | [`germes`] geometry is computed, not listed  |
//! | three divergent alphabets (unicode / enum / glyph key) | [`alphabet`] one canonical table + M2M parser|
//! | library prints to stdout, panics on unknown node       | reports are values, errors are `Result`      |
//!
//! Module map (dependency order):
//!
//! ```text
//! alphabet ─┐
//! germes    │  (pure data / geometry)
//! ast ──────┼─► symbolic ─► contract ─► ┐
//!           │             ownership ──► │
//!           │  relation ◄── derive ◄────┤
//!           │  health  ◄────┘           │
//!           │  lineage                  │
//!           └─ vm                       │
//!                      actor ◄──────────┘
//! ```

pub mod actor;
pub mod alphabet;
pub mod ast;
pub mod contract;
pub mod derive;
pub mod germes;
pub mod health;
pub mod lineage;
pub mod ownership;
pub mod relation;
pub mod symbolic;
pub mod vm;
