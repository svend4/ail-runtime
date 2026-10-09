//! Health metrics of a relation graph.
//!
//! v1's `marginality_ratio` is "fraction of node pairs with ≥1 common
//! neighbour": it is 1.0 for *every* clique from K₃ up and 0.33 for the
//! demo graph, so the documented "≈0.5 is critical" reading has no basis.
//! v2 uses the standard **transitivity** (global clustering coefficient)
//! `3·triangles / wedges` — a true "closed structures / degrees of freedom"
//! ratio in `[0,1]` — plus the lever balance from v1, recomputed on degrees
//! that are now derived.

use super::alphabet::{Degree, Kind};
use super::relation::{NodeKind, RelationGraph};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lever {
    pub clique_pressure: f64,
    pub indep_pressure: f64,
    pub d2_share: f64,
    pub product: f64,
    pub tilted: bool,
}

impl Lever {
    pub fn from_counts(d1: usize, d2: usize, d3: usize) -> Self {
        let total = (d1 + d2 + d3).max(1) as f64;
        let c_raw = d3 as f64 / total;
        let i_raw = d1 as f64 / total;
        let d2_share = d2 as f64 / total;
        let s = (c_raw + i_raw).max(1e-9);
        let c = c_raw / s;
        let i = i_raw / s;
        let product = c * i;
        // With no D1/D3 mass at all the lever is undefined, not tilted.
        let tilted = (d1 + d3) > 0 && ((c - i).abs() >= 0.25 || product <= 0.15);
        Self {
            clique_pressure: c,
            indep_pressure: i,
            d2_share,
            product,
            tilted,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Sparse,
    Critical,
    Rigid,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Health {
    pub nodes: usize,
    pub edges: usize,
    pub d1: usize,
    pub d2: usize,
    pub d3: usize,
    /// `⬡` membership edges, reported separately (scaffolding, not relations).
    pub hyper_edges: usize,
    pub triangles: usize,
    pub wedges: usize,
    pub transitivity: f64,
    pub mean_saturation: f64,
    pub lever: Lever,
    pub zone: Zone,
}

pub fn health(g: &RelationGraph) -> Health {
    // simple graph over non-module nodes for triangle counting
    let ids: Vec<_> = g
        .nodes()
        .filter(|n| n.kind != NodeKind::Module)
        .map(|n| n.id)
        .collect();
    let nb = |id| -> BTreeSet<_> {
        g.neighbors(id)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| {
                g.node(*n)
                    .map(|x| x.kind != NodeKind::Module)
                    .unwrap_or(false)
            })
            .collect()
    };
    let mut triangles = 0usize;
    let mut wedges = 0usize;
    for &v in &ids {
        let n = nb(v);
        let d = n.len();
        wedges += d * d.saturating_sub(1) / 2;
        let nv: Vec<_> = n.iter().copied().collect();
        for i in 0..nv.len() {
            for j in (i + 1)..nv.len() {
                if nb(nv[i]).contains(&nv[j]) {
                    triangles += 1;
                }
            }
        }
    }
    triangles /= 3; // each triangle counted once per vertex
    let transitivity = if wedges == 0 {
        0.0
    } else {
        3.0 * triangles as f64 / wedges as f64
    };
    let relations: Vec<_> = g
        .edges()
        .iter()
        .filter(|e| e.kind != Kind::HyperMember)
        .collect();
    let hyper_edges = g.edges().len() - relations.len();
    let (d1, d2, d3) = degree_counts_of(relations.iter().map(|e| &e.degree));
    let edges = relations.len();
    let mean_saturation = if edges == 0 {
        0.0
    } else {
        relations.iter().map(|e| e.saturation).sum::<f64>() / edges as f64
    };
    let lever = Lever::from_counts(d1, d2, d3);
    let zone = if d2 >= d1 && d2 >= d3 {
        Zone::Critical
    } else if d1 > d3 {
        Zone::Sparse
    } else {
        Zone::Rigid
    };
    Health {
        nodes: g.node_count(),
        edges,
        d1,
        d2,
        d3,
        hyper_edges,
        triangles,
        wedges,
        transitivity,
        mean_saturation,
        lever,
        zone,
    }
}

impl Health {
    pub fn render(&self, title: &str) -> String {
        format!(
            "=== Health: {title} ===\n  nodes/edges      : {}/{} (+{} hyper)\n  D1 / D2 / D3     : {} / {} / {}\n  triangles/wedges : {}/{}  transitivity={:.3}\n  mean saturation  : {:.3}\n  lever C/I        : {:.2}/{:.2}  product={:.3}  {}\n  zone             : {:?}\n",
            self.nodes,
            self.edges,
            self.hyper_edges,
            self.d1,
            self.d2,
            self.d3,
            self.triangles,
            self.wedges,
            self.transitivity,
            self.mean_saturation,
            self.lever.clique_pressure,
            self.lever.indep_pressure,
            self.lever.product,
            if self.lever.tilted { "TILTED" } else { "balanced" },
            self.zone
        )
    }
}

pub fn degree_counts_of<'a>(it: impl Iterator<Item = &'a Degree>) -> (usize, usize, usize) {
    let mut c = (0, 0, 0);
    for d in it {
        match d {
            Degree::D1 => c.0 += 1,
            Degree::D2 => c.1 += 1,
            Degree::D3 => c.2 += 1,
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::super::alphabet::{Kind, Symbol};
    use super::super::relation::NodeKind;
    use super::*;

    #[test]
    fn triangle_has_transitivity_one_and_path_zero() {
        let mut g = RelationGraph::new();
        let a = g.add_node("a", NodeKind::Function);
        let b = g.add_node("b", NodeKind::Function);
        let c = g.add_node("c", NodeKind::Function);
        g.link(a, b, Symbol::Simple, Kind::Data, 1.0).unwrap();
        g.link(b, c, Symbol::Simple, Kind::Data, 1.0).unwrap();
        let h = health(&g);
        assert_eq!((h.triangles, h.wedges), (0, 1));
        assert_eq!(h.transitivity, 0.0);
        g.link(c, a, Symbol::Simple, Kind::Data, 1.0).unwrap();
        let h = health(&g);
        assert_eq!((h.triangles, h.wedges), (1, 3));
        assert_eq!(h.transitivity, 1.0);
    }

    #[test]
    fn lever_is_undefined_not_tilted_when_everything_is_d2() {
        let l = Lever::from_counts(0, 5, 0);
        assert!(!l.tilted);
        assert_eq!(l.d2_share, 1.0);
        let l = Lever::from_counts(4, 0, 0);
        assert!(l.tilted);
    }
}
