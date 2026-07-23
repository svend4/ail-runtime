//! Steps 1–4: Actor owns Graph, risk-aware hot_swap, weighted saturation, marginality metric

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

/// Runtime health snapshot taken after structural changes.
#[derive(Clone, Debug)]
pub struct RuntimeMetrics {
    pub generation: u32,
    pub marginality: f64,
    pub edge_count: usize,
    pub d1_count: usize,
    pub d2_count: usize,
    pub d3_count: usize,
    pub risk_of_last_swap: Option<SwapRisk>,
}

impl RuntimeMetrics {
    pub fn print(&self) {
        println!("=== Runtime Metrics (gen {}) ===", self.generation);
        println!("  marginality_ratio : {:.3}", self.marginality);
        println!("  edges             : {}", self.edge_count);
        println!("  D1 / D2 / D3      : {} / {} / {}", self.d1_count, self.d2_count, self.d3_count);
        if let Some(r) = &self.risk_of_last_swap {
            println!("  last swap risk    : {:?}", r);
        }
        // interpret marginality
        let zone = if self.marginality < 0.35 {
            "sparse (D1-leaning)"
        } else if self.marginality <= 0.65 {
            "near-critical / jamming (D2)"
        } else {
            "over-saturated (D3-leaning)"
        };
        println!("  zone interpretation: {}", zone);
        println!();
    }
}

pub struct ShardActorWithGraph {
    pub module: BinaryModule,
    pub history: Vec<BinaryModule>,
    pub relations: Graph,
    pub fn_nodes: HashMap<String, NodeId>,
    next_node_id: u64,
    /// History of metrics after each accepted swap
    pub metrics_log: Vec<RuntimeMetrics>,
}

impl ShardActorWithGraph {
    pub fn new(module: BinaryModule) -> Self {
        let mut actor = Self {
            module,
            history: Vec::new(),
            relations: Graph::new(),
            fn_nodes: HashMap::new(),
            next_node_id: 1,
            metrics_log: Vec::new(),
        };
        actor.rebuild_relation_graph();
        actor.record_metrics(None);
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
                "  {from_name} -> {to_name} | {:?} | sat={:.3} | {:?} | w={:.2}",
                e.symbol, e.saturation, e.degree, e.weight
            );
        }
        println!();
    }

    // ------------------------------------------------------------------
    // Step 4: metrics
    // ------------------------------------------------------------------

    fn collect_metrics(&self, last_risk: Option<SwapRisk>) -> RuntimeMetrics {
        let mut d1 = 0;
        let mut d2 = 0;
        let mut d3 = 0;
        for e in &self.relations.edges {
            match e.degree {
                ConnectivityDegree::D1 => d1 += 1,
                ConnectivityDegree::D2 => d2 += 1,
                ConnectivityDegree::D3 => d3 += 1,
            }
        }

        RuntimeMetrics {
            generation: self.module.generation,
            marginality: self.relations.marginality_ratio(),
            edge_count: self.relations.edges.len(),
            d1_count: d1,
            d2_count: d2,
            d3_count: d3,
            risk_of_last_swap: last_risk,
        }
    }

    fn record_metrics(&mut self, last_risk: Option<SwapRisk>) {
        let m = self.collect_metrics(last_risk);
        m.print();
        self.metrics_log.push(m);
    }

    pub fn latest_metrics(&self) -> Option<&RuntimeMetrics> {
        self.metrics_log.last()
    }

    // ------------------------------------------------------------------
    // Risk-aware hot_swap (Steps 2 + 4)
    // ------------------------------------------------------------------

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
        // 1. Weakening check
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
                println!("  warning: High risk — full ownership re-check recommended");
            }
            SwapRisk::Medium | SwapRisk::Low => {}
        }

        // 3. Accept
        let candidate = BinaryModule {
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
        self.rebuild_relation_graph();

        println!(
            "Hot-swap accepted: gen {} → gen {}",
            self.module.generation - 1,
            self.module.generation
        );

        // 4. Record runtime metrics (Step 4)
        self.record_metrics(Some(risk));

        Ok(())
    }
}

pub fn run_steps_1_to_4_demo() {
    println!("=== Steps 1–4 Demo: Graph + Risk + Weighted sat + Metrics ===\n");

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

    // Initial relations with different weights (pheromone)
    actor
        .link_functions("Transfer", "CheckBalance", StarGateSymbol::Directed, EdgeKind::Data, 1.0)
        .unwrap();
    actor
        .link_functions("Transfer", "Refund", StarGateSymbol::Conflict, EdgeKind::Ownership, 3.0)
        .unwrap(); // strong conflict
    actor
        .link_functions("Refund", "CheckBalance", StarGateSymbol::Unique, EdgeKind::Ownership, 1.5)
        .unwrap();

    actor.print_relations();
    if let Some(m) = actor.latest_metrics() {
        // already printed at construction; show again after links
        let _ = m;
    }
    // refresh metrics after linking
    actor.record_metrics(None);

    // Compatible swap
    println!("1. Compatible hot-swap:");
    let _ = actor.hot_swap(
        vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "Refund".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        "compatible update",
    );

    // Re-declare links after rebuild
    actor
        .link_functions("Transfer", "Refund", StarGateSymbol::Conflict, EdgeKind::Ownership, 3.0)
        .unwrap();
    actor.record_metrics(None);

    // Try removing Refund
    println!("2. Try remove Refund:");
    let bad = actor.hot_swap(
        vec![
            BinaryFunction { name: "Transfer".into() },
            BinaryFunction { name: "CheckBalance".into() },
        ],
        "remove Refund",
    );
    println!("   result: {:?}\n", bad);

    println!("Metrics log size: {}", actor.metrics_log.len());
    println!("Steps 1–4 complete.");
}
