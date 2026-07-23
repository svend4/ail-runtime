//! Integration demo: Binary AST Actor + Connectivity Degree
//!
//! Shows how saturation / D1-D2-D3 influence hot-swap decisions
//! and how a relation graph between functions can be maintained.

use ail_mvp::connectivity::*;

/// Build a small relation graph for the Payments module.
///
/// Transfer  ✕  Refund     (conflict, expected D3)
/// Transfer  →  CheckBalance
/// CheckBalance — Balance
fn build_payments_relation_graph() -> Graph {
    let mut g = Graph::new();

    let transfer = NodeId(1);
    let refund = NodeId(2);
    let check = NodeId(3);
    let balance = NodeId(4);

    g.add_node(transfer);
    g.add_node(refund);
    g.add_node(check);
    g.add_node(balance);

    // First add weaker links so the conflict edge sees common context
    g.add_edge(
        transfer,
        check,
        StarGateSymbol::Directed,
        EdgeKind::Data,
        1.0,
    );
    g.add_edge(
        check,
        balance,
        StarGateSymbol::Simple,
        EdgeKind::Ownership,
        1.0,
    );
    g.add_edge(
        refund,
        balance,
        StarGateSymbol::Unique,
        EdgeKind::Ownership,
        1.0,
    );

    // Conflict edge between Transfer and Refund
    g.add_edge(
        transfer,
        refund,
        StarGateSymbol::Conflict,
        EdgeKind::Ownership,
        1.0,
    );

    g.recompute_all_saturations();
    g
}

fn print_edge_report(g: &Graph) {
    println!("=== Relation Graph Report ===");
    for e in &g.edges {
        println!(
            "  {:?} -> {:?} | symbol={:?} | sat={:.3} | degree={:?}",
            e.from, e.to, e.symbol, e.saturation, e.degree
        );
    }
    println!("  marginality_ratio = {:.3}", g.marginality_ratio());
    println!();
}

/// Simulate a hot-swap decision that consults connectivity risk.
fn decide_hot_swap(
    relation_graph: &Graph,
    changing_from: NodeId,
    changing_to: NodeId,
    new_symbol: StarGateSymbol,
) -> Result<(), String> {
    // Find old edge if any
    let old = relation_graph
        .edges
        .iter()
        .find(|e| e.from == changing_from && e.to == changing_to);

    // Build a temporary new edge description
    let temp_sat = relation_graph.saturation(changing_from, changing_to);
    let new_degree = Graph::assign_degree(temp_sat);

    let new_edge = Edge {
        from: changing_from,
        to: changing_to,
        symbol: new_symbol,
        kind: EdgeKind::Ownership,
        saturation: temp_sat,
        degree: new_degree,
        weight: 1.0,
    };

    let risk = match old {
        Some(old_e) => hot_swap_risk(old_e, &new_edge),
        None => SwapRisk::Low, // new edge
    };

    println!(
        "Hot-swap risk for {:?} -> {:?}: {:?}",
        changing_from, changing_to, risk
    );

    match risk {
        SwapRisk::Low | SwapRisk::Medium => Ok(()),
        SwapRisk::High => Err("High risk: full weakening + ownership re-check required".into()),
        SwapRisk::Critical => Err("Critical risk: swap rejected by connectivity policy".into()),
    }
}

pub fn run_integration_demo() {
    println!("=== Connectivity + Actor Integration Demo ===\n");

    let g = build_payments_relation_graph();
    print_edge_report(&g);

    // Example 1: try to relax the conflict edge (D3 -> something weaker)
    println!("1. Attempt to downgrade Transfer ✕ Refund (should be Critical/High):");
    match decide_hot_swap(&g, NodeId(1), NodeId(2), StarGateSymbol::Simple) {
        Ok(()) => println!("   Allowed"),
        Err(e) => println!("   Rejected: {}", e),
    }

    // Example 2: modify a D2-ish data edge
    println!("\n2. Attempt to change Transfer → CheckBalance:");
    match decide_hot_swap(&g, NodeId(1), NodeId(3), StarGateSymbol::Directed) {
        Ok(()) => println!("   Allowed (or medium risk)");
        Err(e) => println!("   Rejected: {}", e),
    }

    println!("\n=== Integration demo finished ===");
}
