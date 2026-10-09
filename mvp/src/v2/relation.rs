//! Relation graph with a monotone, three-component saturation model.
//!
//! ## Why v1's formula had to go
//!
//! v1: `s = 2·common_weight / (wdeg(u) + wdeg(v))`, where `wdeg` *includes*
//! the edge (u,v) itself. Consequences (reproduced in the audit):
//!
//! * reinforcing an edge raises the denominator only ⇒ saturation **drops**
//!   (triangle: 0.50 → 0.14 after `reinforce(+5)`), the opposite of the docs;
//! * for a clique Kₙ the value is `(n−2)/(n−1)` ⇒ D3 is unreachable below K₆,
//!   so "`✕` ⇒ D3 (s ≈ 0.9)" never holds in any example graph;
//! * a declared `✕` with no shared neighbours is D1 ⇒ `can_borrow_unique`
//!   allows a unique borrow across a conflict.
//!
//! ## v2 model
//!
//! Every edge has three named components, each in `[0,1]`:
//!
//! ```text
//! topology(u,v) = J_w(u,v) · (1 − exp(−C_w(u,v) / support_scale))
//!     J_w  = weighted Jaccard of neighbourhoods N(u)∖{v}, N(v)∖{u}
//!     C_w  = Σ_x min(w(u,x), w(v,x))           (weighted common neighbours)
//! usage(e)      = 1 − exp(−max(w_e − 1, 0) / usage_scale)      (pheromone)
//! prior(e)      = symbol prior from the canonical alphabet       (declared)
//!
//! saturation(e) = 1 − (1 − topology)(1 − usage)(1 − prior)       (noisy-OR)
//! ```
//!
//! Properties (all tested below): bounded in `[0,1]`; **monotone** in each
//! component, so reinforcing an edge never lowers its saturation; a declared
//! conflict is D3 on its own; a path edge with no context and default weight
//! is D1; the module hyper-node can be excluded from topology so grouping does
//! not inflate every intra-module edge.

use super::alphabet::{degree_for, Degree, Kind, Modifier, Statement, Symbol};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub usize);

/// Vertex kinds from `docs/edge-rules.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeKind {
    Domain,
    Module,
    Function,
    Entity,
    Detail,
    Article,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub id: NodeId,
    pub label: String,
    pub kind: NodeKind,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SatComponents {
    pub topology: f64,
    pub usage: f64,
    pub prior: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub symbol: Symbol,
    pub kind: Kind,
    pub weight: f64,
    /// `%` modifier: pinned to D3.
    pub forced_dense: bool,
    pub components: SatComponents,
    pub saturation: f64,
    pub degree: Degree,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SaturationPolicy {
    pub d2_low: f64,
    pub d3_low: f64,
    pub use_prior: bool,
    pub usage_scale: f64,
    pub support_scale: f64,
    pub exclude_modules_from_topology: bool,
}

impl Default for SaturationPolicy {
    fn default() -> Self {
        Self {
            d2_low: 0.25,
            d3_low: 0.75,
            use_prior: true,
            usage_scale: 3.0,
            support_scale: 2.0,
            exclude_modules_from_topology: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RelationError {
    UnknownNode(NodeId),
    UnknownEdge(EdgeId),
    UnknownLabel(String),
    SelfLoop(NodeId),
    DuplicateEdge {
        from: NodeId,
        to: NodeId,
        kind: Kind,
    },
    KindNotAllowed {
        symbol: Symbol,
        kind: Kind,
    },
    NegativeWeight(f64),
}

impl fmt::Display for RelationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Clone, Debug, Default)]
pub struct RelationGraph {
    nodes: BTreeMap<NodeId, Node>,
    by_label: BTreeMap<String, NodeId>,
    /// undirected adjacency: neighbour → edge ids between the pair
    adj: BTreeMap<NodeId, BTreeMap<NodeId, Vec<EdgeId>>>,
    edges: Vec<Edge>,
    next: u64,
    pub policy: SaturationPolicy,
}

impl RelationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: SaturationPolicy) -> Self {
        Self {
            policy,
            ..Default::default()
        }
    }

    // -- nodes ------------------------------------------------------------

    /// Add a node, or return the existing node with this label.
    pub fn add_node(&mut self, label: &str, kind: NodeKind) -> NodeId {
        if let Some(id) = self.by_label.get(label) {
            return *id;
        }
        self.next += 1;
        let id = NodeId(self.next);
        self.nodes.insert(
            id,
            Node {
                id,
                label: label.to_string(),
                kind,
            },
        );
        self.by_label.insert(label.to_string(), id);
        self.adj.entry(id).or_default();
        id
    }

    pub fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    pub fn node_by_label(&self, label: &str) -> Option<NodeId> {
        self.by_label.get(label).copied()
    }

    pub fn label(&self, id: NodeId) -> &str {
        self.nodes.get(&id).map(|n| n.label.as_str()).unwrap_or("?")
    }

    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.values()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn neighbors(&self, id: NodeId) -> Result<BTreeSet<NodeId>, RelationError> {
        self.adj
            .get(&id)
            .map(|m| m.keys().copied().collect())
            .ok_or(RelationError::UnknownNode(id))
    }

    // -- edges ------------------------------------------------------------

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id.0)
    }

    pub fn edges_between(&self, a: NodeId, b: NodeId) -> Vec<EdgeId> {
        self.adj
            .get(&a)
            .and_then(|m| m.get(&b))
            .cloned()
            .unwrap_or_default()
    }

    pub fn incident(&self, id: NodeId) -> Vec<EdgeId> {
        self.adj
            .get(&id)
            .map(|m| m.values().flatten().copied().collect())
            .unwrap_or_default()
    }

    /// Add an edge. One edge per `(pair, kind)`; use [`reinforce`](Self::reinforce) to strengthen.
    pub fn link(
        &mut self,
        from: NodeId,
        to: NodeId,
        symbol: Symbol,
        kind: Kind,
        weight: f64,
    ) -> Result<EdgeId, RelationError> {
        if !self.nodes.contains_key(&from) {
            return Err(RelationError::UnknownNode(from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(RelationError::UnknownNode(to));
        }
        if from == to {
            return Err(RelationError::SelfLoop(from));
        }
        if !symbol.allows(kind) {
            return Err(RelationError::KindNotAllowed { symbol, kind });
        }
        if weight < 0.0 {
            return Err(RelationError::NegativeWeight(weight));
        }
        if self
            .edges_between(from, to)
            .iter()
            .any(|e| self.edges[e.0].kind == kind)
        {
            return Err(RelationError::DuplicateEdge { from, to, kind });
        }
        let id = EdgeId(self.edges.len());
        self.edges.push(Edge {
            id,
            from,
            to,
            symbol,
            kind,
            weight,
            forced_dense: false,
            components: SatComponents {
                topology: 0.0,
                usage: 0.0,
                prior: 0.0,
            },
            saturation: 0.0,
            degree: Degree::D1,
        });
        self.adj
            .entry(from)
            .or_default()
            .entry(to)
            .or_default()
            .push(id);
        self.adj
            .entry(to)
            .or_default()
            .entry(from)
            .or_default()
            .push(id);
        self.recompute();
        Ok(id)
    }

    pub fn link_labels(
        &mut self,
        from: &str,
        to: &str,
        symbol: Symbol,
        kind: Kind,
        weight: f64,
    ) -> Result<EdgeId, RelationError> {
        let f = self
            .node_by_label(from)
            .ok_or_else(|| RelationError::UnknownLabel(from.into()))?;
        let t = self
            .node_by_label(to)
            .ok_or_else(|| RelationError::UnknownLabel(to.into()))?;
        self.link(f, t, symbol, kind, weight)
    }

    /// Pheromone update; weight never drops below zero.
    pub fn reinforce(&mut self, id: EdgeId, delta: f64) -> Result<(), RelationError> {
        let e = self
            .edges
            .get_mut(id.0)
            .ok_or(RelationError::UnknownEdge(id))?;
        e.weight = (e.weight + delta).max(0.0);
        self.recompute();
        Ok(())
    }

    /// Multiply every weight by `factor` (time decay).
    pub fn decay(&mut self, factor: f64) {
        for e in &mut self.edges {
            e.weight = (e.weight * factor).max(0.0);
        }
        self.recompute();
    }

    pub fn force_dense(&mut self, id: EdgeId, on: bool) -> Result<(), RelationError> {
        let e = self
            .edges
            .get_mut(id.0)
            .ok_or(RelationError::UnknownEdge(id))?;
        e.forced_dense = on;
        self.recompute();
        Ok(())
    }

    // -- saturation -------------------------------------------------------

    fn pair_weight(&self, a: NodeId, b: NodeId) -> f64 {
        self.edges_between(a, b)
            .iter()
            .map(|e| self.edges[e.0].weight)
            .sum()
    }

    fn topology_neighbors(&self, id: NodeId, exclude: NodeId) -> BTreeSet<NodeId> {
        self.adj
            .get(&id)
            .map(|m| {
                m.keys()
                    .copied()
                    .filter(|n| *n != exclude)
                    .filter(|n| {
                        !(self.policy.exclude_modules_from_topology
                            && self.nodes[n].kind == NodeKind::Module)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Weighted-Jaccard × support. See module docs.
    pub fn topology(&self, u: NodeId, v: NodeId) -> f64 {
        let nu = self.topology_neighbors(u, v);
        let nv = self.topology_neighbors(v, u);
        let union: BTreeSet<NodeId> = nu.union(&nv).copied().collect();
        if union.is_empty() {
            return 0.0;
        }
        let mut num = 0.0;
        let mut den = 0.0;
        for x in &union {
            let a = if nu.contains(x) {
                self.pair_weight(u, *x)
            } else {
                0.0
            };
            let b = if nv.contains(x) {
                self.pair_weight(v, *x)
            } else {
                0.0
            };
            num += a.min(b);
            den += a.max(b);
        }
        if den <= 0.0 {
            return 0.0;
        }
        let jaccard = num / den;
        let support = 1.0 - (-num / self.policy.support_scale).exp();
        (jaccard * support).clamp(0.0, 1.0)
    }

    pub fn usage(&self, weight: f64) -> f64 {
        (1.0 - (-(weight - 1.0).max(0.0) / self.policy.usage_scale).exp()).clamp(0.0, 1.0)
    }

    pub fn combine(&self, c: SatComponents) -> f64 {
        let prior = if self.policy.use_prior { c.prior } else { 0.0 };
        (1.0 - (1.0 - c.topology) * (1.0 - c.usage) * (1.0 - prior)).clamp(0.0, 1.0)
    }

    pub fn degree_for(&self, saturation: f64) -> Degree {
        degree_for(saturation, self.policy.d2_low, self.policy.d3_low)
    }

    pub fn recompute(&mut self) {
        let comps: Vec<SatComponents> = self
            .edges
            .iter()
            .map(|e| SatComponents {
                topology: self.topology(e.from, e.to),
                usage: self.usage(e.weight),
                prior: e.symbol.prior(),
            })
            .collect();
        for (e, c) in self.edges.iter_mut().zip(comps) {
            let prior = if self.policy.use_prior { c.prior } else { 0.0 };
            let s = (1.0 - (1.0 - c.topology) * (1.0 - c.usage) * (1.0 - prior)).clamp(0.0, 1.0);
            e.components = c;
            e.saturation = s;
            e.degree = if e.forced_dense {
                Degree::D3
            } else {
                degree_for(s, self.policy.d2_low, self.policy.d3_low)
            };
        }
    }

    pub fn degree_counts(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for e in &self.edges {
            match e.degree {
                Degree::D1 => c.0 += 1,
                Degree::D2 => c.1 += 1,
                Degree::D3 => c.2 += 1,
            }
        }
        c
    }

    // -- M2M --------------------------------------------------------------

    /// Serialise to the M2M line format (hyper groups first).
    pub fn to_m2m(&self) -> String {
        let mut out = String::new();
        for n in self.nodes.values().filter(|n| n.kind == NodeKind::Module) {
            let members: Vec<String> = self
                .incident(n.id)
                .iter()
                .map(|e| &self.edges[e.0])
                .filter(|e| e.kind == Kind::HyperMember)
                .map(|e| if e.from == n.id { e.to } else { e.from })
                .map(|m| self.label(m).to_string())
                .collect();
            if !members.is_empty() {
                out.push_str(
                    &Statement::Hyper {
                        label: n.label.clone(),
                        members,
                    }
                    .to_m2m(),
                );
                out.push('\n');
            }
        }
        for e in &self.edges {
            if e.kind == Kind::HyperMember {
                continue;
            }
            let st = Statement::Relation {
                from: self.label(e.from).to_string(),
                symbol: e.symbol,
                to: self.label(e.to).to_string(),
                modifiers: if e.forced_dense {
                    vec![Modifier::Dense]
                } else {
                    vec![]
                },
            };
            out.push_str(&st.to_m2m());
            out.push('\n');
        }
        out
    }

    /// Build a graph from M2M text. Commands are returned untouched for the caller.
    pub fn from_m2m(
        text: &str,
        policy: SaturationPolicy,
    ) -> Result<(RelationGraph, Vec<Statement>), String> {
        let stmts = super::alphabet::parse(text).map_err(|e| e.to_string())?;
        let mut g = RelationGraph::with_policy(policy);
        let mut commands = Vec::new();
        for st in stmts {
            match st {
                Statement::Relation {
                    from,
                    symbol,
                    to,
                    modifiers,
                } => {
                    let f = g.add_node(&from, NodeKind::Other);
                    let t = g.add_node(&to, NodeKind::Other);
                    let kind = symbol.spec().kinds[0];
                    let id = g.link(f, t, symbol, kind, 1.0).map_err(|e| e.to_string())?;
                    if modifiers.contains(&Modifier::Dense) {
                        g.force_dense(id, true).map_err(|e| e.to_string())?;
                    }
                }
                Statement::Hyper { label, members } => {
                    let m = g.add_node(&label, NodeKind::Module);
                    for member in members {
                        let n = g.add_node(&member, NodeKind::Other);
                        g.link(m, n, Symbol::Hyper, Kind::HyperMember, 1.0)
                            .map_err(|e| e.to_string())?;
                    }
                }
                Statement::Command { .. } => commands.push(st),
                Statement::Empty => {}
            }
        }
        Ok((g, commands))
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        for e in &self.edges {
            s.push_str(&format!(
                "  {:<14} {} {:<14} | {:?}/{:?} | w={:.2} | topo={:.2} use={:.2} prior={:.2} | sat={:.3} {:?}{}\n",
                self.label(e.from),
                e.symbol,
                self.label(e.to),
                e.symbol,
                e.kind,
                e.weight,
                e.components.topology,
                e.components.usage,
                e.components.prior,
                e.saturation,
                e.degree,
                if e.forced_dense { " (forced)" } else { "" }
            ));
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle(sym: Symbol, kind: Kind) -> (RelationGraph, [NodeId; 3], [EdgeId; 3]) {
        let mut g = RelationGraph::new();
        let a = g.add_node("a", NodeKind::Function);
        let b = g.add_node("b", NodeKind::Function);
        let c = g.add_node("c", NodeKind::Function);
        let ab = g.link(a, b, sym, kind, 1.0).unwrap();
        let bc = g.link(b, c, sym, kind, 1.0).unwrap();
        let ca = g.link(c, a, sym, kind, 1.0).unwrap();
        (g, [a, b, c], [ab, bc, ca])
    }

    #[test]
    fn reinforce_never_lowers_saturation() {
        let (mut g, _, [ab, ..]) = triangle(Symbol::Simple, Kind::Data);
        let mut prev = g.edge(ab).unwrap().saturation;
        for _ in 0..10 {
            g.reinforce(ab, 1.0).unwrap();
            let s = g.edge(ab).unwrap().saturation;
            assert!(s >= prev - 1e-12, "saturation dropped {prev} → {s}");
            prev = s;
        }
        assert!(prev > 0.8);
    }

    #[test]
    fn declared_conflict_is_d3_even_without_context() {
        let mut g = RelationGraph::new();
        let a = g.add_node("Transfer", NodeKind::Function);
        let b = g.add_node("Refund", NodeKind::Function);
        let e = g
            .link(a, b, Symbol::Conflict, Kind::Ownership, 1.0)
            .unwrap();
        assert_eq!(g.edge(e).unwrap().degree, Degree::D3);
        // and a bare simple edge is D1
        let c = g.add_node("Log", NodeKind::Function);
        let e2 = g.link(a, c, Symbol::Simple, Kind::Data, 1.0).unwrap();
        assert_eq!(g.edge(e2).unwrap().degree, Degree::D1);
    }

    #[test]
    fn topology_grows_with_clique_size_and_is_bounded() {
        let mut last = 0.0;
        for n in 3..=8u64 {
            let mut g = RelationGraph::new();
            let ids: Vec<NodeId> = (0..n)
                .map(|i| g.add_node(&format!("n{i}"), NodeKind::Function))
                .collect();
            for i in 0..n as usize {
                for j in (i + 1)..n as usize {
                    g.link(ids[i], ids[j], Symbol::Simple, Kind::Data, 1.0)
                        .unwrap();
                }
            }
            let t = g.edge(EdgeId(0)).unwrap().components.topology;
            assert!(t > last && t <= 1.0, "K{n}: {t}");
            last = t;
        }
        // K3 / K4 stay in the working zone, K5+ become rigid on topology alone
        let deg = |n: u64| {
            let mut g = RelationGraph::with_policy(SaturationPolicy {
                use_prior: false,
                ..Default::default()
            });
            let ids: Vec<NodeId> = (0..n)
                .map(|i| g.add_node(&format!("n{i}"), NodeKind::Function))
                .collect();
            for i in 0..n as usize {
                for j in (i + 1)..n as usize {
                    g.link(ids[i], ids[j], Symbol::Simple, Kind::Data, 1.0)
                        .unwrap();
                }
            }
            g.edge(EdgeId(0)).unwrap().degree
        };
        assert_eq!(deg(3), Degree::D2);
        assert_eq!(deg(4), Degree::D2);
        assert_eq!(deg(5), Degree::D3);
    }

    #[test]
    fn module_node_does_not_inflate_topology() {
        let mut g = RelationGraph::new();
        let m = g.add_node("Mod", NodeKind::Module);
        let a = g.add_node("a", NodeKind::Function);
        let b = g.add_node("b", NodeKind::Function);
        g.link(m, a, Symbol::Hyper, Kind::HyperMember, 1.0).unwrap();
        g.link(m, b, Symbol::Hyper, Kind::HyperMember, 1.0).unwrap();
        let e = g.link(a, b, Symbol::Simple, Kind::Data, 1.0).unwrap();
        assert_eq!(g.edge(e).unwrap().components.topology, 0.0);
        assert_eq!(g.edge(e).unwrap().degree, Degree::D1);
    }

    #[test]
    fn errors_instead_of_panics() {
        let mut g = RelationGraph::new();
        let a = g.add_node("a", NodeKind::Function);
        assert_eq!(
            g.neighbors(NodeId(99)),
            Err(RelationError::UnknownNode(NodeId(99)))
        );
        assert_eq!(
            g.link(a, NodeId(99), Symbol::Simple, Kind::Data, 1.0),
            Err(RelationError::UnknownNode(NodeId(99)))
        );
        assert_eq!(
            g.link(a, a, Symbol::Simple, Kind::Data, 1.0),
            Err(RelationError::SelfLoop(a))
        );
        let b = g.add_node("b", NodeKind::Function);
        assert_eq!(
            g.link(a, b, Symbol::Conflict, Kind::Control, 1.0),
            Err(RelationError::KindNotAllowed {
                symbol: Symbol::Conflict,
                kind: Kind::Control
            })
        );
        g.link(a, b, Symbol::Simple, Kind::Data, 1.0).unwrap();
        assert_eq!(
            g.link(a, b, Symbol::Directed, Kind::Data, 1.0),
            Err(RelationError::DuplicateEdge {
                from: a,
                to: b,
                kind: Kind::Data
            })
        );
        assert_eq!(
            g.reinforce(EdgeId(7), 1.0),
            Err(RelationError::UnknownEdge(EdgeId(7)))
        );
    }

    #[test]
    fn decay_and_force_dense() {
        let (mut g, _, [ab, ..]) = triangle(Symbol::Simple, Kind::Data);
        g.reinforce(ab, 9.0).unwrap();
        let hi = g.edge(ab).unwrap().saturation;
        g.decay(0.1);
        assert!(g.edge(ab).unwrap().saturation < hi);
        g.force_dense(ab, true).unwrap();
        assert_eq!(g.edge(ab).unwrap().degree, Degree::D3);
    }

    #[test]
    fn m2m_round_trip() {
        let text = "PaymentsModule # Transfer Refund CheckBalance\nTransfer ) Refund\nTransfer ! CheckBalance\nRefund \" Balance %\nBal\n";
        let (g, cmds) = RelationGraph::from_m2m(text, SaturationPolicy::default()).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(g.edges().len(), 6);
        let out = g.to_m2m();
        let (g2, _) = RelationGraph::from_m2m(&out, SaturationPolicy::default()).unwrap();
        assert_eq!(g2.to_m2m(), out);
        let forced = g2.edges().iter().find(|e| e.forced_dense).unwrap();
        assert_eq!(forced.degree, Degree::D3);
    }
}
