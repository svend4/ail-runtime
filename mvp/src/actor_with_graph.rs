//! Step 1 + Step 2: ShardActor owns Graph and hot_swap uses risk matrix

use ail_mvp::connectivity::*;
use std::collections::HashMap;

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

pub struct ShardActorWithGraph {
    pub module: BinaryModule,
    pub history: Vec<BinaryModule>,
    pub relations: Graph,
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

    pub fn rebuild_relation_graph(&mut self) {
        self.relations = Graph::new();
        self.fn_nodes.clear();

        for func in &self.module.functions {
            let nid = self.alloc_node();
            self.relations.add_node(nid);
            self.fn_nodes.insert(func.name.clone(), nid);
        }
    }

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

    pub fn print_relations(&self) {
        println!(
            "=== Relations of module `{}` (gen {}) ===",
            self.module.name, self.module.generation
        );
        for e in &self.relations.edges {
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

    // ------------------------------------------------------------------
    // Step 2: connectivity-aware hot_swap
    // ------------------------------------------------------------------

    /// Estimate risk of replacing the current module with `new_functions`.
    ///
    /// Current policy (MVP):
    /// - if any existing D3 edge would be affected by a removed function → Critical
    /// - if any D2 edge is affected → High
    /// - otherwise Medium/Low depending on remaining structure
    fn estimate_swap_risk(&self, new_functions: &[BinaryFunction]) -> SwapRisk {
        let new_names: std::collections::HashSet<&str> =
            new_functions.iter().map(|f| f.name.as_str()).collect();

        let mut worst = SwapRisk::Low;

        for e in &self.relations.edges {
            let from_name = self
                .fn_nodes
                .iter()
                .find(|(_, id)| **id == e.from)
                .map(|(n, _)| n.as_str());
            let to_name = self
                .fn_nodes
                .iter()
                .find(|(_, id)| **id == e.to)
                .map(|(n, _)| n.as_str());

            let affected = match (from_name, to_name) {
                (Some(a), Some(b)) => !new_names.contains(a) || !new_names.contains(b),
                _ => false,
            };

            if !affected {
                continue;
            }

            let risk = match e.degree {
                ConnectivityDegree::D1 => SwapRisk::Low,
                ConnectivityDegree::D2 => SwapRisk::High,
                ConnectivityDegree::D3 => SwapRisk::Critical,
            };

            if risk > worst {
                worst = risk;
            }
        }

        worst
    }

    pub fn hot_swap(
        &mut self,
        new_functions: Vec<BinaryFunction>,
        reason: &str,
    ) -> Result<(), String> {
        // 1. Classic weakening: no function may disappear
        for old_fn in &self.module.functions {
            if !new_functions.iter().any(|f| f.name == old_fn.name) {
                return Err(format!(
                    "Function `{}` was removed — weakening detected (gen {} → gen {})",
                    old_fn.name, self.module.generation, self.module.generation + 1
                ));
            }
        }

        // 2. Connectivity risk
        let risk = self.estimate_swap_risk(&new_functions);
        println!("  connectivity risk: {:?}", risk);

        match risk {
            SwapRisk::Critical => {
                return Err("Critical connectivity risk — swap rejected".into());
            }
            SwapRisk::High => {
                println!("  warning: High risk — would require full ownership re-check in production");
                // For MVP we still allow, but log loudly
            }
            SwapRisk::Medium | SwapRisk::Low => {}
        }

        // 3. Accept
        let mut candidate = BinaryModule {
            name: self.module.name.clone(),
            version: self.module.version + 1,
            functions: new_functions,
            content_hash: format!("hash-gen-{}", self.module.generation + 1),
            parent_hash: Some(self.module.content_hash.clone()),
            generation: self.module.generation + 1,
            origin: ModuleOrigin::Healed {
                reason: reason.to_string(),
                from_generation: self.module.generation,
            },
        };

        self.history.push(self.module.clone());
        self.module = candidate;

        // Rebuild graph for the new generation (nodes may stay, edges re-declared by caller)
        self.rebuild_relation_graph();

        println!(
            "Hot-swap accepted: gen {} → gen {}",
            self.module.generation - 1,
            self.module.generation
        );
        Ok(())
    }
}

pub fn run_step1_and_step2_demo() {
    println!("=== Step 1+2: Actor owns Graph + risk-aware hot_swap ===\n");

    let module = BinaryModule {
        name: "PaymentsModule".into(),
        version: 1,
        functions: vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "Refund".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        content_hash: "gen0".into(),
        parent_hash: None,
        generation: 0,
        origin: ModuleOrigin::Manual,
    };

    let mut actor = ShardActorWithGraph::new(module);

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

    // Compatible swap (all functions kept)
    println!("1. Compatible hot-swap (all functions kept):");
    let ok = actor.hot_swap(
        vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "Refund".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        "compatible update",
    );
    println!("   result: {:?}\n", ok);

    // Re-link after rebuild
    actor
        .link_functions("Transfer", "Refund", StarGateSymbol::Conflict, EdgeKind::Ownership, 1.0)
        .unwrap();

    // Attempt to remove Refund while a D3 conflict edge exists
    println!("2. Try to remove Refund (should hit Critical risk or weakening):");
    let bad = actor.hot_swap(
        vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        "remove Refund",
    );
    println!("   result: {:?}\n", bad);

    println!("Steps 1 and 2 complete.");
}
