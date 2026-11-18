//! AIL Runtime MVP library.
//!
//! Contains two parallel layers inside a single `ail_mvp` crate:
//! - Legacy **v1** modules (`connectivity`, `vizgraph`, `lever`), preserved
//!   unchanged for backward compatibility with the `ail-mvp` binary.
//! - Audited **v2** semantic runtime (`v2`), providing typed AST (`v2::ast`),
//!   symbolic effect extraction (`v2::symbolic`), contract refinement checks
//!   (`v2::contract`), static ownership analysis (`v2::ownership`), alias-checked
//!   VM execution (`v2::vm`), derived relation graphs (`v2::derive`,
//!   `v2::relation`), graph health metrics (`v2::health`), Merkle lineage
//!   (`v2::lineage`), and hot-swap orchestration (`v2::actor`).

pub mod connectivity;
pub mod vizgraph;
pub mod lever;

pub use connectivity::*;
pub use vizgraph::*;
pub use lever::*;

// Additive build fix (audit branch): modules inside this crate refer to it by
// its own name (`ail_mvp::...`). Register the crate under that name so those
// paths resolve without rewriting the modules.
extern crate self as ail_mvp;

pub mod v2;
