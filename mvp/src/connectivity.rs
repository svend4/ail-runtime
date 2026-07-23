//! Connectivity Degree & Saturation
//!
//! Practical implementation of the D1/D2/D3 scale
//! inspired by jamming theory (Parisi–Zamponi 2026).

use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Basic types
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConnectivityDegree {
    D1, // sparse / open
    D2, // jamming / fluid (main working zone)
    D3, // rigid / saturated
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StarGateSymbol {
    Dot,
    Simple,     // —
    Directed,   // →
    Angle,      // ∠
    Triangle,   // △
    Diamond,    // ◇
    Shared,     // □̸
    Unique,     // ▲
    Conflict,   // ✕
    Hyper,      // ⬡
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    Control,
    Data,
    Ownership,
    HyperMember,
    Diff,
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub symbol: StarGateSymbol,
    pub kind: EdgeKind,
    pub saturation: f64,
    pub degree: ConnectivityDegree,
    pub weight: f32,
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct Graph {
    nodes: HashSet<NodeId>,
    // adjacency list: node → set of neighbours
    adj: HashMap<NodeId, HashSet<NodeId>>,
    edges: Vec<Edge>,
}

impl Graph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, id: NodeId) {
        self.nodes.insert(id);
        self.adj.entry(id).or_default();
    }

    pub fn neighbors(&self, id: NodeId) -> &HashSet<NodeId> {
        self.adj.get(&id).expect("node must exist")
    }

    pub fn degree_of(&self, id: NodeId) -> usize {
        self.neighbors(id).len()
    }

    /// Compute saturation for a potential or existing edge (u, v)
    pub fn saturation(&self, u: NodeId, v: NodeId) -> f64 {
        let nu = self.neighbors(u);
        let nv = self.neighbors(v);
        let common = nu.intersection(nv).count();
        let du = nu.len();
        let dv = nv.len();
        if du + dv == 0 {
            return 0.0;
        }
        2.0 * common as f64 / (du + dv) as f64
    }

    pub fn assign_degree(s: f64) -> ConnectivityDegree {
        if s < 0.25 {
            ConnectivityDegree::D1
        } else if s <= 0.75 {
            ConnectivityDegree::D2
        } else {
            ConnectivityDegree::D3
        }
    }

    /// Add an edge and automatically compute saturation + degree
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        symbol: StarGateSymbol,
        kind: EdgeKind,
        weight: f32,
    ) -> &Edge {
        self.add_node(from);
        self.add_node(to);

        // temporarily add adjacency so saturation sees the new link's context
        self.adj.get_mut(&from).unwrap().insert(to);
        self.adj.get_mut(&to).unwrap().insert(from);

        let s = self.saturation(from, to);
        let degree = Self::assign_degree(s);

        let edge = Edge {
            from,
            to,
            symbol,
            kind,
            saturation: s,
            degree,
            weight,
        };
        self.edges.push(edge);
        self.edges.last().unwrap()
    }

    /// Recompute saturation for all edges (call after bulk changes)
    pub fn recompute_all_saturations(&mut self) {
        // We need a two-phase approach because we cannot borrow self mutably
        // while iterating edges. Collect new values first.
        let updates: Vec<(usize, f64, ConnectivityDegree)> = self
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let s = self.saturation(e.from, e.to);
                let d = Self::assign_degree(s);
                (i, s, d)
            })
            .collect();

        for (i, s, d) in updates {
            self.edges[i].saturation = s;
            self.edges[i].degree = d;
        }
    }

    // -----------------------------------------------------------------------
    // Marginality ratio (subgraph diagnostic)
    // -----------------------------------------------------------------------

    /// Very simple N/D approximation on the whole graph
    pub fn marginality_ratio(&self) -> f64 {
        let mut shared_pairs = 0usize;
        let mut possible = 0usize;

        let nodes: Vec<NodeId> = self.nodes.iter().copied().collect();
        for i in 0..nodes.len() {
            for j in (i + 1)..nodes.len() {
                possible += 1;
                let u = nodes[i];
                let v = nodes[j];
                let common = self.neighbors(u).intersection(self.neighbors(v)).count();
                if common > 0 {
                    shared_pairs += 1;
                }
            }
        }

        if possible == 0 {
            return 0.0;
        }
        shared_pairs as f64 / possible as f64
    }
}

// ---------------------------------------------------------------------------
// Ownership helpers (degree-aware)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum BorrowError {
    ConflictOnD3,
    UniqueNotAllowedOnD2,
    UniqueNotAllowedOnD3,
}

pub fn can_borrow_shared(edge: &Edge) -> bool {
    // Shared borrow is almost always allowed; D3 only warns
    true
}

pub fn can_borrow_unique(edge: &Edge) -> Result<(), BorrowError> {
    match edge.degree {
        ConnectivityDegree::D1 => Ok(()),
        ConnectivityDegree::D2 => {
            // allowed but should be treated carefully by the caller
            Ok(())
        }
        ConnectivityDegree::D3 => Err(BorrowError::UniqueNotAllowedOnD3),
    }
}

// ---------------------------------------------------------------------------
// Hot-Swap risk
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SwapRisk {
    Low,
    Medium,
    High,
    Critical,
}

pub fn hot_swap_risk(old: &Edge, new: &Edge) -> SwapRisk {
    use ConnectivityDegree::*;
    match (old.degree, new.degree) {
        (D1, D1) | (D1, D2) => SwapRisk::Low,
        (D1, D3) => SwapRisk::Medium,
        (D2, D1) | (D2, D2) => SwapRisk::Medium,
        (D2, D3) => SwapRisk::High,
        (D3, _) => SwapRisk::Critical,
    }
}

// ---------------------------------------------------------------------------
// Demo / self-check
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saturation_and_degree_basic() {
        let mut g = Graph::new();
        let a = NodeId(1);
        let b = NodeId(2);
        let c = NodeId(3);

        g.add_node(a);
        g.add_node(b);
        g.add_node(c);

        // triangle a-b-c will produce higher saturation
        g.add_edge(a, b, StarGateSymbol::Simple, EdgeKind::Data, 1.0);
        g.add_edge(b, c, StarGateSymbol::Simple, EdgeKind::Data, 1.0);
        g.add_edge(c, a, StarGateSymbol::Simple, EdgeKind::Data, 1.0);

        g.recompute_all_saturations();

        for e in &g.edges {
            assert!(e.saturation >= 0.0 && e.saturation <= 1.0);
            // in a triangle saturation becomes relatively high
            assert!(e.degree == ConnectivityDegree::D2 || e.degree == ConnectivityDegree::D3);
        }
    }
}
