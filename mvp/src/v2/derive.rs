//! Derive the relation graph from the Binary AST.
//!
//! "Saturation is computed, not declared" (docs/saturation-usage.md §8) —
//! but in v1 the *edges* are declared by hand (`link_functions`), so the
//! conflict `Transfer ✕ Refund` exists only because a demo typed it. v2 reads
//! it off the code via symbolic effect footprints:
//!
//! | overlap between f and g (same nominal type + field)   | edge              |
//! |-------------------------------------------------------|-------------------|
//! | both write                                            | `✕` Conflict      |
//! | one writes, the other reads                           | `▲` Unique (w→r)  |
//! | both only read                                        | `□̸` Shared        |
//! | f calls g                                             | `→` Directed      |
//! | module ∋ f                                            | `⬡` HyperMember   |
//!
//! Edge weight = number of overlapping `(type, field)` locations, so a pair
//! that collides on three fields is "heavier" than one colliding on one.

use super::alphabet::{Kind, Symbol};
use super::ast::Module;
use super::contract::{contract_of, Contract};
use super::relation::{NodeId, NodeKind, RelationGraph, SaturationPolicy};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Derived {
    pub graph: RelationGraph,
    pub fn_nodes: BTreeMap<String, NodeId>,
    pub module_node: NodeId,
    pub contracts: BTreeMap<String, Contract>,
}

pub fn derive(m: &Module) -> Derived {
    derive_with(m, SaturationPolicy::default())
}

pub fn derive_with(m: &Module, policy: SaturationPolicy) -> Derived {
    let mut g = RelationGraph::with_policy(policy);
    let module_node = g.add_node(&m.name, NodeKind::Module);
    let mut fn_nodes = BTreeMap::new();
    let mut contracts = BTreeMap::new();

    for f in &m.functions {
        let id = g.add_node(&f.name, NodeKind::Function);
        fn_nodes.insert(f.name.clone(), id);
        contracts.insert(f.name.clone(), contract_of(f));
        // hyper membership
        let _ = g.link(module_node, id, Symbol::Hyper, Kind::HyperMember, 1.0);
    }

    let fns = &m.functions;
    for (i, f) in fns.iter().enumerate() {
        let cf = &contracts[&f.name];
        for g2 in fns.iter().skip(i + 1) {
            let cg = &contracts[&g2.name];
            let ef = &cf.effects;
            let eg = &cg.effects;
            let ww = ef.typed_writes.intersection(&eg.typed_writes).count();
            let f_writes_g_reads = ef.typed_writes.intersection(&eg.typed_reads).count();
            let g_writes_f_reads = eg.typed_writes.intersection(&ef.typed_reads).count();
            let rr = ef.typed_reads.intersection(&eg.typed_reads).count();
            let (a, b) = (fn_nodes[&f.name], fn_nodes[&g2.name]);
            if ww > 0 {
                let _ = g.link(a, b, Symbol::Conflict, Kind::Ownership, ww as f64);
            } else if f_writes_g_reads + g_writes_f_reads > 0 {
                let (from, to) = if f_writes_g_reads >= g_writes_f_reads {
                    (a, b)
                } else {
                    (b, a)
                };
                let _ = g.link(
                    from,
                    to,
                    Symbol::Unique,
                    Kind::Ownership,
                    (f_writes_g_reads + g_writes_f_reads) as f64,
                );
            } else if rr > 0 {
                let _ = g.link(a, b, Symbol::Shared, Kind::Ownership, rr as f64);
            }
        }
        // control edges
        for callee in &cf.effects.calls {
            if let Some(&to) = fn_nodes.get(callee) {
                let from = fn_nodes[&f.name];
                if from != to {
                    let _ = g.link(from, to, Symbol::Directed, Kind::Control, 1.0);
                }
            }
        }
    }

    Derived {
        graph: g,
        fn_nodes,
        module_node,
        contracts,
    }
}

impl Derived {
    pub fn node_of(&self, name: &str) -> Option<NodeId> {
        self.fn_nodes.get(name).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::super::alphabet::Degree;
    use super::super::ast::examples::*;
    use super::*;

    #[test]
    fn transfer_conflicts_with_refund_without_anyone_declaring_it() {
        let d = derive(&payments_module());
        let t = d.node_of("Transfer").unwrap();
        let r = d.node_of("Refund").unwrap();
        let c = d.node_of("CheckBalance").unwrap();
        let tr = d.graph.edges_between(t, r);
        assert_eq!(tr.len(), 1);
        let e = d.graph.edge(tr[0]).unwrap();
        assert_eq!(e.symbol, Symbol::Conflict);
        assert_eq!(e.degree, Degree::D3);
        // writer → reader is a unique borrow edge
        let tc = d.graph.edges_between(t, c);
        let e = d.graph.edge(tc[0]).unwrap();
        assert_eq!(e.symbol, Symbol::Unique);
        assert_eq!((e.from, e.to), (t, c));
        assert_eq!(e.degree, Degree::D2);
    }

    #[test]
    fn call_becomes_a_control_edge() {
        let d = derive(&payments_module_v2());
        let t = d.node_of("Transfer").unwrap();
        let c = d.node_of("CheckBalance").unwrap();
        let kinds: Vec<Kind> = d
            .graph
            .edges_between(t, c)
            .iter()
            .map(|e| d.graph.edge(*e).unwrap().kind)
            .collect();
        assert!(kinds.contains(&Kind::Control));
        assert!(kinds.contains(&Kind::Ownership));
    }

    #[test]
    fn module_hyper_edges_are_present_and_serialisable() {
        let d = derive(&payments_module());
        let m2m = d.graph.to_m2m();
        assert!(m2m.starts_with("PaymentsModule # "));
        assert!(m2m.contains("Transfer ) Refund"));
    }
}
