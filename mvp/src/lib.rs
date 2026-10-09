//! AIL Runtime MVP library

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
