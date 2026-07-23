//! Demo for Step 5: build a VizScene from the actor relation graph

use ail_mvp::connectivity::*;
use ail_mvp::vizgraph::VizGraph;
use std::collections::HashMap;

use crate::actor_with_graph::{BinaryFunction, BinaryModule, ModuleOrigin, ShardActorWithGraph};

pub fn run_viz_demo() {
    println!("=== Step 5: VizGraph Demo ===\n");

    let module = BinaryModule {
        name: "PaymentsModule".into(),
        version: 1,
        functions: vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "Refund".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        content_hash: "viz0".into(),
        parent_hash: None,
        generation: 0,
        origin: ModuleOrigin::Manual,
    };

    let mut actor = ShardActorWithGraph::new(module);

    actor
        .link_functions("Transfer", "CheckBalance", StarGateSymbol::Directed, EdgeKind::Data, 1.0)
        .unwrap();
    actor
        .link_functions("Transfer", "Refund", StarGateSymbol::Conflict, EdgeKind::Ownership, 3.0)
        .unwrap();
    actor
        .link_functions("Refund", "CheckBalance", StarGateSymbol::Unique, EdgeKind::Ownership, 1.5)
        .unwrap();

    // Build label map NodeId → name
    let labels: HashMap<NodeId, String> = actor
        .fn_nodes
        .iter()
        .map(|(name, id)| (*id, name.clone()))
        .collect();

    let scene = VizGraph::from_graph(&actor.relations, &labels, "PaymentsModule relations");
    VizGraph::print_scene(&scene);

    println!("Step 5 complete: scene ready for terminal / JSON / canvas renderer.");
}
