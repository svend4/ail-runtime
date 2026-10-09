//! AIL Runtime v2 demo — everything derived from code, nothing declared.
//!
//! Run: `cargo run --bin v2-demo`

use ail_mvp::v2::actor::{Decision, ShardActor};
use ail_mvp::v2::alphabet::{Symbol, TABLE};
use ail_mvp::v2::ast::examples::*;
use ail_mvp::v2::ast::{Module, Op, Reg};
use ail_mvp::v2::germes;
use ail_mvp::v2::ownership;
use ail_mvp::v2::relation::{RelationGraph, SaturationPolicy};
use ail_mvp::v2::vm::Value;

fn banner(t: &str) {
    println!("\n======== {t} ========");
}

fn main() {
    println!("=== AIL Runtime v2 Demo (audit branch) ===");

    banner("1. Canonical Star Gate alphabet");
    println!(
        "  {:<9} {:<3} {:<4} {:<5} {:<4} kinds",
        "name", "uni", "key", "prior", "deg"
    );
    for s in TABLE.iter() {
        println!(
            "  {:<9} {:<3} {:<4} {:<5.2} {:<4} {:?}",
            s.name,
            s.unicode,
            s.glyph_key.map(|c| c.to_string()).unwrap_or("-".into()),
            s.prior,
            s.symbol.prior_degree(),
            s.kinds
        );
    }

    banner("2. GERMES hexagon — computed, not listed");
    print!("{}", germes::render_ascii());
    let errs = germes::antipode_table_errors();
    println!(
        "  docs/germes-mapping.md antipode table: {} of 3 pairs are not antipodes",
        errs.len()
    );
    for ((a, b), fix) in errs {
        println!(
            "    {} ↔ {}  is wrong (60° apart); {} ↔ {} is the antipode",
            a.code(),
            b.code(),
            a.code(),
            fix.code()
        );
    }

    banner("3. Derived relation graph of PaymentsModule");
    let mut actor = ShardActor::new(payments_module());
    print!("{}", actor.derived.graph.render());
    println!("  (Transfer ✕ Refund is derived: both write Wallet.balance; nobody declared it)");
    print!("{}", actor.health().render("gen 0"));
    println!("M2M export:\n{}", indent(&actor.m2m()));

    banner("4. Static ownership analysis + runtime alias check");
    for f in &actor.module.functions {
        let r = ownership::analyze(f);
        println!(
            "  {:<13} errors={} obligations={:?}",
            f.name,
            r.errors.len(),
            r.obligations
                .iter()
                .map(|o| o.to_string())
                .collect::<Vec<_>>()
        );
    }
    let x = actor.heap.alloc(WALLET);
    let y = actor.heap.alloc(WALLET);
    actor.heap.set_field(x, BALANCE, Value::I64(1000)).unwrap();
    actor.heap.set_field(y, BALANCE, Value::I64(200)).unwrap();
    println!(
        "  Transfer(x, y, 300)  → {:?}",
        actor.call(
            "Transfer",
            vec![Value::Obj(x), Value::Obj(y), Value::I64(300)]
        )
    );
    println!(
        "    x={:?} y={:?}",
        actor.heap.field_i64(x, BALANCE),
        actor.heap.field_i64(y, BALANCE)
    );
    println!(
        "  Transfer(x, y, 9999) → {:?}",
        actor.call(
            "Transfer",
            vec![Value::Obj(x), Value::Obj(y), Value::I64(9999)]
        )
    );
    println!(
        "  Transfer(x, x, 300)  → {:?}",
        actor.call(
            "Transfer",
            vec![Value::Obj(x), Value::Obj(x), Value::I64(300)]
        )
    );
    println!(
        "    x={:?}  (v1 would have accepted the aliased call)",
        actor.heap.field_i64(x, BALANCE)
    );

    banner("5. Hot-swap: weakened Transfer (guard dropped, name kept)");
    let mut weak = payments_module();
    weak.functions[0] = transfer_without_guard();
    match actor.swap(weak, "drop guard") {
        Ok(r) => print!("{}", r.render()),
        Err(r) => print!("{}", r.render()),
    }

    banner("6. Hot-swap: semantic change on a D3 edge (fee), contract intact");
    let mut fee = payments_module();
    {
        let f = &mut fee.functions[0];
        f.regs = 11;
        f.body.insert(
            6,
            Op::ConstI64 {
                value: 1,
                dest: Reg(9),
            },
        );
        f.body.insert(
            7,
            Op::SubChecked {
                a: Reg(2),
                b: Reg(9),
                dest: Reg(10),
            },
        );
        if let Op::AddChecked { b, .. } = &mut f.body[8] {
            *b = Reg(10);
        }
    }
    let r = actor.propose(&fee);
    print!("{}", r.render());

    banner("7. Hot-swap: compatible update (Transfer delegates to CheckBalance)");
    let r = actor.propose(&payments_module_v2());
    print!("{}", r.render());
    println!("  → the pure callee is inlined, guards compare equal; only call structure changed (Medium, accepted).");

    banner("8. Hot-swap: no-op generation + Merkle lineage");
    let r = actor
        .swap(payments_module(), "no-op redeploy")
        .expect("accepted");
    println!("  decision={:?} risk={:?}", r.decision, r.risk);
    print!("{}", actor.lineage.render(&actor.module.name));
    println!("  verify_lineage = {:?}", actor.verify_lineage());
    println!("  (same content hash, distinct generation ids — v1 produced identical hashes)");

    banner("9. Saturation model sanity");
    let mut g = RelationGraph::with_policy(SaturationPolicy::default());
    let a = g.add_node("a", ail_mvp::v2::relation::NodeKind::Function);
    let b = g.add_node("b", ail_mvp::v2::relation::NodeKind::Function);
    let c = g.add_node("c", ail_mvp::v2::relation::NodeKind::Function);
    let ab = g
        .link(a, b, Symbol::Simple, ail_mvp::v2::alphabet::Kind::Data, 1.0)
        .unwrap();
    g.link(b, c, Symbol::Simple, ail_mvp::v2::alphabet::Kind::Data, 1.0)
        .unwrap();
    g.link(c, a, Symbol::Simple, ail_mvp::v2::alphabet::Kind::Data, 1.0)
        .unwrap();
    println!(
        "  triangle a—b before reinforce: sat={:.3} {:?}",
        g.edge(ab).unwrap().saturation,
        g.edge(ab).unwrap().degree
    );
    g.reinforce(ab, 5.0).unwrap();
    println!(
        "  triangle a—b after  reinforce(+5): sat={:.3} {:?}   (v1: 0.500 → 0.143)",
        g.edge(ab).unwrap().saturation,
        g.edge(ab).unwrap().degree
    );

    let _ = Module {
        name: String::new(),
        functions: vec![],
    };
    let _ = Decision::Accept;
    println!("\n=== v2 demo finished ===");
}

fn indent(s: &str) -> String {
    s.lines()
        .map(|l| format!("    {l}"))
        .collect::<Vec<_>>()
        .join("\n")
}
