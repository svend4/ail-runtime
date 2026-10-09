//! Lineage as a Merkle chain.
//!
//! v1 hashes only `(name, functions)`; a compatible hot-swap therefore
//! produces a child whose `content_hash` equals its parent's, and
//! `parent_hash` points at an identical string (visible in the v1 demo:
//! `gen 0 … e55418a48857` → `gen 1 … e55418a48857`). v2 separates
//! *content identity* (what the module is) from *generation identity*
//! (where it sits in history): `id = H(content_hash ‖ parent_id ‖ generation)`.

use super::ast::Module;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Origin {
    Manual,
    Loaded,
    Healed {
        reason: String,
        from_generation: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generation {
    pub generation: u32,
    /// Hash of the module content only (content-addressed).
    pub content_hash: String,
    /// Chain id of the parent generation.
    pub parent_id: Option<String>,
    /// Chain id of this generation.
    pub id: String,
    pub origin: Origin,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Canonical content hash: SHA-256 over the canonical serialisation of the
/// module. Struct field order is fixed by the type, so this is deterministic
/// (v1 hashed the `to_string()` of a `serde_json::Value`, which is also
/// deterministic but tied to the JSON representation of the *debug* shape).
pub fn content_hash(m: &Module) -> String {
    let bytes = serde_json::to_vec(m).expect("module is serialisable");
    let mut h = Sha256::new();
    h.update(b"ail/v2/module\0");
    h.update(&bytes);
    hex(&h.finalize())
}

pub fn chain_id(content_hash: &str, parent_id: Option<&str>, generation: u32) -> String {
    let mut h = Sha256::new();
    h.update(b"ail/v2/generation\0");
    h.update(content_hash.as_bytes());
    h.update(b"\0");
    h.update(parent_id.unwrap_or("").as_bytes());
    h.update(b"\0");
    h.update(generation.to_be_bytes());
    hex(&h.finalize())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LineageError {
    Empty,
    GenerationGap {
        at: usize,
        expected: u32,
        found: u32,
    },
    BrokenParentLink {
        at: usize,
    },
    IdMismatch {
        at: usize,
    },
    ContentMismatch {
        at: usize,
    },
}

impl fmt::Display for LineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lineage {
    pub entries: Vec<Generation>,
}

impl Lineage {
    pub fn genesis(m: &Module, origin: Origin) -> Self {
        let ch = content_hash(m);
        let id = chain_id(&ch, None, 0);
        Self {
            entries: vec![Generation {
                generation: 0,
                content_hash: ch,
                parent_id: None,
                id,
                origin,
            }],
        }
    }

    pub fn head(&self) -> Option<&Generation> {
        self.entries.last()
    }

    pub fn append(&mut self, m: &Module, origin: Origin) -> &Generation {
        let (gen, parent) = match self.head() {
            Some(h) => (h.generation + 1, Some(h.id.clone())),
            None => (0, None),
        };
        let ch = content_hash(m);
        let id = chain_id(&ch, parent.as_deref(), gen);
        self.entries.push(Generation {
            generation: gen,
            content_hash: ch,
            parent_id: parent,
            id,
            origin,
        });
        self.entries.last().unwrap()
    }

    /// Verify the chain structure (ids and links) without the modules.
    pub fn verify_links(&self) -> Result<(), LineageError> {
        if self.entries.is_empty() {
            return Err(LineageError::Empty);
        }
        for (i, g) in self.entries.iter().enumerate() {
            if g.generation != i as u32 {
                return Err(LineageError::GenerationGap {
                    at: i,
                    expected: i as u32,
                    found: g.generation,
                });
            }
            let parent = if i == 0 {
                None
            } else {
                Some(self.entries[i - 1].id.as_str())
            };
            if g.parent_id.as_deref() != parent {
                return Err(LineageError::BrokenParentLink { at: i });
            }
            if g.id != chain_id(&g.content_hash, parent, g.generation) {
                return Err(LineageError::IdMismatch { at: i });
            }
        }
        Ok(())
    }

    /// Verify links *and* that each stored module matches its content hash.
    pub fn verify(&self, modules: &[Module]) -> Result<(), LineageError> {
        self.verify_links()?;
        for (i, (g, m)) in self.entries.iter().zip(modules).enumerate() {
            if g.content_hash != content_hash(m) {
                return Err(LineageError::ContentMismatch { at: i });
            }
        }
        Ok(())
    }

    pub fn render(&self, module_name: &str) -> String {
        let mut s = format!("=== Lineage of `{module_name}` ===\n");
        for g in &self.entries {
            let origin = match &g.origin {
                Origin::Manual => "Manual".to_string(),
                Origin::Loaded => "Loaded".to_string(),
                Origin::Healed {
                    reason,
                    from_generation,
                } => format!("Healed from gen {from_generation} ({reason})"),
            };
            s.push_str(&format!(
                "gen {:>2} | id {} | content {} | {}\n",
                g.generation,
                &g.id[..12],
                &g.content_hash[..12],
                origin
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::*;

    #[test]
    fn identical_content_gets_distinct_generation_ids() {
        let m = payments_module();
        let mut l = Lineage::genesis(&m, Origin::Manual);
        l.append(
            &m,
            Origin::Healed {
                reason: "no-op".into(),
                from_generation: 0,
            },
        );
        assert_eq!(l.entries[0].content_hash, l.entries[1].content_hash);
        assert_ne!(l.entries[0].id, l.entries[1].id);
        assert!(l.verify(&[m.clone(), m]).is_ok());
    }

    #[test]
    fn tampering_is_detected() {
        let m = payments_module();
        let mut l = Lineage::genesis(&m, Origin::Manual);
        l.append(&payments_module_v2(), Origin::Loaded);
        assert!(l.verify(&[m.clone(), payments_module_v2()]).is_ok());
        // swap the stored module
        assert_eq!(
            l.verify(&[m.clone(), m.clone()]),
            Err(LineageError::ContentMismatch { at: 1 })
        );
        // break a link
        let mut bad = l.clone();
        bad.entries[1].parent_id = Some("deadbeef".into());
        assert_eq!(
            bad.verify_links(),
            Err(LineageError::BrokenParentLink { at: 1 })
        );
        let mut bad = l.clone();
        bad.entries[1].content_hash = "00".repeat(32);
        assert_eq!(bad.verify_links(), Err(LineageError::IdMismatch { at: 1 }));
    }

    #[test]
    fn content_hash_is_stable_and_sensitive() {
        assert_eq!(
            content_hash(&payments_module()),
            content_hash(&payments_module())
        );
        assert_ne!(
            content_hash(&payments_module()),
            content_hash(&payments_module_v2())
        );
    }
}
