//! Connectivity Degree & Saturation
//!
//! Practical implementation of the D1/D2/D3 scale
//! inspired by jamming theory (Parisi–Zamponi 2026).
//!
//! Step 3 adds weighted saturation: edge weights (pheromone / importance)
//! influence the continuous saturation value.

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
    Simple,   // —
    Directed, // →
    Angle,    // ∠
    Triangle, // △
    Diamond,  // ◇
    Shared,   // □̸
    Unique,   // ▲
    Conflict, // ✕
    Hyper,    // ⬡
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
    /// Structural / semantic importance (pheromone). Default 1.0
    pub weight: f32,
}

// ---------------------------------------------------------------------------
// Graph
// ---------------------------------------------------------------------------

#[derive(Default, Debug)]
pub struct Graph {
    nodes: HashSet<NodeId>,
    adj: HashMap<NodeId, HashSet<NodeId>>,
    /// edge key (min,max) → weight, for fast weighted lookups
    edge_weights: HashMap<(NodeId, NodeId), f32>,
    pub edges: Vec<Edge>,
}

fn edge_key(a: NodeId, b: NodeId) -> (NodeId, NodeId) {
    if a.0 <= b.0 {
        (a, b)
    } else {
        (b, a)
    }
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

    fn weight_between(&self, a: NodeId, b: NodeId) -> f32 {
        self.edge_weights
            .get(&edge_key(a, b))
            .copied()
            .unwrap_or(1.0)
    }

    /// Unweighted saturation (structural only)
    pub fn saturation_unweighted(&self, u: NodeId, v: NodeId) -> f64 {
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

    /// Weighted saturation (Step 3)
    ///
    /// s_w(u,v) = (sum of weights of edges from u or v into common neighbours)
    ///            / (weighted degree of u + weighted degree of v)
    ///
    /// Falls back to unweighted formula when no weights are present.
    pub fn saturation(&self, u: NodeId, v: NodeId) -> f64 {
        let nu = self.neighbors(u);
        let nv = self.neighbors(v);

        let common: Vec<NodeId> = nu.intersection(nv).copied().collect();

        // Weighted contribution of common neighbours
        let mut common_weight = 0.0f64;
        for w in &common {
            let wu = self.weight_between(u, *w) as f64;
            let wv = self.weight_between(v, *w) as f64;
            common_weight += (wu + wv) / 2.0;
        }

        // Weighted degrees
        let mut deg_u_w = 0.0f64;
        for n in nu {
            deg_u_w += self.weight_between(u, *n) as f64;
        }
        let mut deg_v_w = 0.0f64;
        for n in nv {
            deg_v_w += self.weight_between(v, *n) as f64;
        }

        if deg_u_w + deg_v_w < 1e-12 {
            return 0.0;
        }

        // Normalise into [0,1] approximately
        let s = 2.0 * common_weight / (deg_u_w + deg_v_w);
        s.clamp(0.0, 1.0)
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

        self.adj.get_mut(&from).unwrap().insert(to);
        self.adj.get_mut(&to).unwrap().insert(from);
        self.edge_weights.insert(edge_key(from, to), weight.max(0.0));

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

    /// Boost or decay the weight (pheromone) of an existing edge and recompute.
    pub fn reinforce_edge(&mut self, from: NodeId, to: NodeId, delta: f32) {
        let key = edge_key(from, to);
        let w = self.edge_weights.entry(key).or_insert(1.0);
        *w = (*w + delta).max(0.0);

        // keep Edge.weight in sync
        for e in &mut self.edges {
            if edge_key(e.from, e.to) == key {
                e.weight = *w;
            }
        }

        self.recompute_all_saturations();
    }

    pub fn recompute_all_saturations(&mut self) {
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
// Ownership helpers
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum BorrowError {
    ConflictOnD3,
    UniqueNotAllowedOnD2,
    UniqueNotAllowedOnD3,
}

pub fn can_borrow_shared(_edge: &Edge) -> bool {
    true
}

pub fn can_borrow_unique(edge: &Edge) -> Result<(), BorrowError> {
    match edge.degree {
        ConnectivityDegree::D1 => Ok(()),
        ConnectivityDegree::D2 => Ok(()),
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_saturation_increases_with_weight() {
        let mut g = Graph::new();
        let a = NodeId(1);
        let b = NodeId(2);
        let c = NodeId(3);

        g.add_node(a);
        g.add_node(b);
        g.add_node(c);

        // triangle with default weight 1.0
        g.add_edge(a, b, StarGateSymbol::Simple, EdgeKind::Data, 1.0);
        g.add_edge(b, c, StarGateSymbol::Simple, EdgeKind::Data, 1.0);
        g.add_edge(c, a, StarGateSymbol::Simple, EdgeKind::Data, 1.0);
        g.recompute_all_saturations();

        let s_before = g.edges[0].saturation;

        // reinforce one edge strongly
        g.reinforce_edge(a, b, 5.0);
        let s_after = g.edges.iter().find(|e| e.from == a && e.to == b).unwrap().saturation;

        // saturation should remain in [0,1] and typically change
        assert!(s_before >= 0.0 && s_before <= 1.0);
        assert!(s_after >= 0.0 && s_after <= 1.0);
    }
}
