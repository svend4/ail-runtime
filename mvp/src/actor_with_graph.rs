//! Step 1: ShardActor owns its relation Graph
//!
//! Every module/actor now carries a connectivity view of its functions
//! and important values. Saturation and degree are recomputed when the
//! relation graph changes.

use ail_mvp::connectivity::*;
use std::collections::HashMap;

// Minimal stand-ins so this file is self-contained for the step.
// In the full MVP these come from main.rs / Binary AST.

#[derive(Clone, Debug)]
pub struct BinaryFunction {
    pub name: String,
}

#[derive(Clone, Debug)]
pub enum ModuleOrigin {
    Manual,
    Healed { reason: String, from_generation: u32 },
}

#[derive(Clone, Debug)]
pub struct BinaryModule {
    pub name: String,
    pub version: u32,
    pub functions: Vec<BinaryFunction>,
    pub content_hash: String,
    pub parent_hash: Option<String>,
    pub generation: u32,
    pub origin: ModuleOrigin,
}

/// Extended actor that owns both the executable module and its relation graph.
pub struct ShardActorWithGraph {
    pub module: BinaryModule,
    pub history: Vec<BinaryModule>,
    /// Connectivity view of functions and key values inside this module
    pub relations: Graph,
    /// Map from function name → NodeId in the relation graph
    pub fn_nodes: HashMap<String, NodeId>,
    next_node_id: u64,
}

impl ShardActorWithGraph {
    pub fn new(module: BinaryModule) -> Self {
        let mut actor = Self {
            module,
            history: Vec::new(),
            relations: Graph::new(),
            fn_nodes: HashMap::new(),
            next_node_id: 1,
        };
        actor.rebuild_relation_graph();
        actor
    }

    fn alloc_node(&mut self) -> NodeId {
        let id = NodeId(self.next_node_id);
        self.next_node_id += 1;
        id
    }

    /// Rebuild the relation graph from the current module functions.
    /// For now we create one node per function and no edges yet.
    /// Edges are added by higher-level policy (conflicts, data-flow, …).
    pub fn rebuild_relation_graph(&mut self) {
        self.relations = Graph::new();
        self.fn_nodes.clear();

        for func in &self.module.functions {
            let nid = self.alloc_node();
            self.relations.add_node(nid);
            self.fn_nodes.insert(func.name.clone(), nid);
        }
    }

    /// Declare a typed relation between two functions.
    pub fn link_functions(
        &mut self,
        from_name: &str,
        to_name: &str,
        symbol: StarGateSymbol,
        kind: EdgeKind,
        weight: f32,
    ) -> Result<(), String> {
        let from = *self
            .fn_nodes
            .get(from_name)
            .ok_or_else(|| format!("unknown function `{from_name}`"))?;
        let to = *self
            .fn_nodes
            .get(to_name)
            .ok_or_else(|| format!("unknown function `{to_name}`"))?;

        self.relations.add_edge(from, to, symbol, kind, weight);
        self.relations.recompute_all_saturations();
        Ok(())
    }

    /// Print current connectivity view.
    pub fn print_relations(&self) {
        println!("=== Relations of module `{}` (gen {}) ===", self.module.name, self.module.generation);
        for e in &self.relations.edges {
            // resolve names for readability
            let from_name = self
                .fn_nodes
                .iter()
                .find(|(_, id)| **id == e.from)
                .map(|(n, _)| n.as_str())
                .unwrap_or("?");
            let to_name = self
                .fn_nodes
                .iter()
                .find(|(_, id)| **id == e.to)
                .map(|(n, _)| n.as_str())
                .unwrap_or("?");

            println!(
                "  {from_name} -> {to_name} | {:?} | sat={:.3} | {:?}",
                e.symbol, e.saturation, e.degree
            );
        }
        println!("  marginality = {:.3}", self.relations.marginality_ratio());
        println!();
    }
}

/// Demo of Step 1
pub fn run_step1_demo() {
    println!("=== Step 1: Relation Graph inside Actor ===\n");

    let module = BinaryModule {
        name: "PaymentsModule".into(),
        version: 1,
        functions: vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "Refund".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        content_hash: "abc".into(),
        parent_hash: None,
        generation: 0,
        origin: ModuleOrigin::Manual,
    };

    let mut actor = ShardActorWithGraph::new(module);

    // Declare semantic relations
    actor
        .link_functions("Transfer", "CheckBalance", StarGateSymbol::Directed, EdgeKind::Data, 1.0)
        .unwrap();
    actor
        .link_functions("Transfer", "Refund", StarGateSymbol::Conflict, EdgeKind::Ownership, 1.0)
        .unwrap();
    actor
        .link_functions("Refund", "CheckBalance", StarGateSymbol::Unique, EdgeKind::Ownership, 1.0)
        .unwrap();

    actor.print_relations();

    println!("Step 1 complete: actor now owns its connectivity view.");
}
