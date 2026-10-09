//! Shard actor v2: module + derived relations + Merkle lineage + heap, with a
//! hot-swap pipeline that returns a *report* instead of printing.
//!
//! Pipeline (every stage runs; nothing short-circuits the diagnostics):
//!
//! ```text
//! propose(new)
//!   1. contract refinement      (contract::check_refinement)
//!   2. static ownership         (ownership::analyze on every new function)
//!   3. connectivity risk        (derived graph of the *current* module,
//!                                edges incident to functions whose contract
//!                                changed or vanished)
//!   4. decision by policy
//! commit(new, report)  → new generation in lineage, graph re-derived,
//!                        health snapshot appended
//! ```
//!
//! In v1 the risk stage could never fire: the name check rejected removals
//! first, and for a non-removal no edge was "affected". Here risk is computed
//! from *semantic* change, so a kept-but-rewritten function on a D3 edge is
//! Critical, while a purely structural change (a new function, a guard
//! delegated to a pure callee) is Medium and never blocks.

use super::alphabet::{Degree, Kind, Symbol};
use super::ast::Module;
use super::contract::{check_refinement, Refinement};
use super::derive::{derive_with, Derived};
use super::health::{health, Health};
use super::lineage::{Generation, Lineage, Origin};
use super::ownership::{analyze, Report as OwnershipReport};
use super::relation::SaturationPolicy;
use super::vm::{Heap, Value, Vm, VmError};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwapPolicy {
    pub reject_on_violation: bool,
    pub reject_on_ownership_error: bool,
    pub reject_on_critical: bool,
    pub saturation: SaturationPolicy,
}

impl Default for SwapPolicy {
    fn default() -> Self {
        Self {
            reject_on_violation: true,
            reject_on_ownership_error: true,
            reject_on_critical: true,
            saturation: SaturationPolicy::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Accept,
    AcceptWithWarnings(Vec<String>),
    Reject(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct AffectedEdge {
    pub from: String,
    pub to: String,
    pub symbol: Symbol,
    pub kind: Kind,
    pub degree: Degree,
    pub saturation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwapReport {
    pub refinement: Refinement,
    pub ownership: Vec<(String, OwnershipReport)>,
    pub touched: BTreeSet<String>,
    pub affected: Vec<AffectedEdge>,
    pub risk: Risk,
    pub decision: Decision,
}

impl SwapReport {
    pub fn accepted(&self) -> bool {
        !matches!(self.decision, Decision::Reject(_))
    }

    pub fn render(&self) -> String {
        let mut s = String::new();
        s.push_str("--- swap report ---\n");
        for v in &self.refinement.violations {
            s.push_str(&format!("  violation : {v}\n"));
        }
        for c in &self.refinement.changes {
            s.push_str(&format!("  change    : {c}\n"));
        }
        for (name, r) in &self.ownership {
            for e in &r.errors {
                s.push_str(&format!("  ownership : `{name}` {e}\n"));
            }
            for o in &r.obligations {
                s.push_str(&format!("  obligation: `{name}` {o}\n"));
            }
        }
        for a in &self.affected {
            s.push_str(&format!(
                "  affected  : {} {} {} ({:?}, sat={:.2}, {:?})\n",
                a.from, a.symbol, a.to, a.kind, a.saturation, a.degree
            ));
        }
        s.push_str(&format!("  risk      : {:?}\n", self.risk));
        s.push_str(&format!("  decision  : {:?}\n", self.decision));
        s
    }
}

pub struct ShardActor {
    pub module: Module,
    pub history: Vec<Module>,
    pub lineage: Lineage,
    pub derived: Derived,
    pub health_log: Vec<Health>,
    pub heap: Heap,
    pub policy: SwapPolicy,
}

impl ShardActor {
    pub fn new(module: Module) -> Self {
        Self::with_policy(module, SwapPolicy::default())
    }

    pub fn with_policy(module: Module, policy: SwapPolicy) -> Self {
        let derived = derive_with(&module, policy.saturation);
        let lineage = Lineage::genesis(&module, Origin::Manual);
        let h = health(&derived.graph);
        Self {
            module,
            history: Vec::new(),
            lineage,
            derived,
            health_log: vec![h],
            heap: Heap::default(),
            policy,
        }
    }

    pub fn generation(&self) -> u32 {
        self.lineage.head().map(|g| g.generation).unwrap_or(0)
    }

    pub fn health(&self) -> &Health {
        self.health_log.last().expect("health log is never empty")
    }

    /// Execute a function against the actor's persistent heap.
    pub fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        let heap = std::mem::take(&mut self.heap);
        let mut vm = Vm::with_heap(&self.module, heap);
        let r = vm.call(name, args);
        self.heap = vm.into_heap();
        r
    }

    /// Run the full pipeline without committing.
    pub fn propose(&self, new: &Module) -> SwapReport {
        let refinement = check_refinement(&self.module, new);
        let ownership: Vec<(String, OwnershipReport)> = new
            .functions
            .iter()
            .map(|f| (f.name.clone(), analyze(f)))
            .collect();

        let touched = refinement.semantically_touched();
        let mut affected = Vec::new();
        for name in &touched {
            if let Some(node) = self.derived.node_of(name) {
                for eid in self.derived.graph.incident(node) {
                    let e = self.derived.graph.edge(eid).unwrap();
                    if e.kind == Kind::HyperMember {
                        continue;
                    }
                    affected.push(AffectedEdge {
                        from: self.derived.graph.label(e.from).to_string(),
                        to: self.derived.graph.label(e.to).to_string(),
                        symbol: e.symbol,
                        kind: e.kind,
                        degree: e.degree,
                        saturation: e.saturation,
                    });
                }
            }
        }
        affected.sort_by(|a, b| {
            (a.from.as_str(), a.to.as_str(), a.kind).cmp(&(b.from.as_str(), b.to.as_str(), b.kind))
        });
        affected.dedup();

        let risk = if touched.is_empty() {
            if refinement.has_structural_changes() {
                Risk::Medium
            } else {
                Risk::Low
            }
        } else {
            affected
                .iter()
                .map(|a| match a.degree {
                    Degree::D1 => Risk::Medium,
                    Degree::D2 => Risk::High,
                    Degree::D3 => Risk::Critical,
                })
                .max()
                .unwrap_or(Risk::Medium)
        };

        let mut reasons = Vec::new();
        let mut warnings = Vec::new();
        if self.policy.reject_on_violation {
            reasons.extend(
                refinement
                    .violations
                    .iter()
                    .map(|v| format!("contract: {v}")),
            );
        } else {
            warnings.extend(
                refinement
                    .violations
                    .iter()
                    .map(|v| format!("contract: {v}")),
            );
        }
        for (name, r) in &ownership {
            for e in &r.errors {
                let msg = format!("ownership: `{name}` {e}");
                if self.policy.reject_on_ownership_error {
                    reasons.push(msg);
                } else {
                    warnings.push(msg);
                }
            }
        }
        match risk {
            Risk::Critical if self.policy.reject_on_critical => {
                reasons.push("connectivity: critical risk on a D3 edge".into())
            }
            Risk::Critical | Risk::High => warnings.push(format!(
                "connectivity: {risk:?} risk — full re-check recommended"
            )),
            _ => {}
        }
        let decision = if !reasons.is_empty() {
            Decision::Reject(reasons)
        } else if !warnings.is_empty() {
            Decision::AcceptWithWarnings(warnings)
        } else {
            Decision::Accept
        };

        SwapReport {
            refinement,
            ownership,
            touched,
            affected,
            risk,
            decision,
        }
    }

    /// Commit a proposal that was accepted.
    pub fn commit(
        &mut self,
        new: Module,
        report: &SwapReport,
        reason: &str,
    ) -> Result<&Generation, String> {
        if !report.accepted() {
            return Err(format!("report was rejected: {:?}", report.decision));
        }
        let from = self.generation();
        let old = std::mem::replace(&mut self.module, new);
        self.history.push(old);
        self.derived = derive_with(&self.module, self.policy.saturation);
        self.health_log.push(health(&self.derived.graph));
        Ok(self.lineage.append(
            &self.module,
            Origin::Healed {
                reason: reason.to_string(),
                from_generation: from,
            },
        ))
    }

    /// Propose and commit in one step. The rejected report is returned whole
    /// (it is the diagnostic the caller wants), hence the large `Err`.
    #[allow(clippy::result_large_err)]
    pub fn swap(&mut self, new: Module, reason: &str) -> Result<SwapReport, SwapReport> {
        let report = self.propose(&new);
        if report.accepted() {
            self.commit(new, &report, reason)
                .expect("accepted report commits");
            Ok(report)
        } else {
            Err(report)
        }
    }

    /// Verify that history, lineage and the live module agree.
    pub fn verify_lineage(&self) -> Result<(), super::lineage::LineageError> {
        let mut modules: Vec<Module> = self.history.clone();
        modules.push(self.module.clone());
        self.lineage.verify(&modules)
    }

    pub fn m2m(&self) -> String {
        self.derived.graph.to_m2m()
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::super::ast::*;
    use super::*;

    #[test]
    fn compatible_swap_is_accepted_and_lineage_verifies() {
        let mut a = ShardActor::new(payments_module());
        let r = a.swap(payments_module(), "no-op").unwrap();
        assert_eq!(r.decision, Decision::Accept);
        assert_eq!(r.risk, Risk::Low);
        assert_eq!(a.generation(), 1);
        assert!(a.verify_lineage().is_ok());
        assert_eq!(a.health_log.len(), 2);
    }

    #[test]
    fn dropping_a_guard_is_rejected_by_contract_not_by_name() {
        let mut a = ShardActor::new(payments_module());
        let mut m = payments_module();
        m.functions[0] = transfer_without_guard();
        let r = a.swap(m, "weaken").unwrap_err();
        assert!(
            matches!(r.decision, Decision::Reject(ref why) if why.iter().any(|w| w.contains("dropped guard")))
        );
        // Transfer sits on the D3 conflict edge: critical risk is reported too
        assert_eq!(r.risk, Risk::Critical);
        assert_eq!(a.generation(), 0);
    }

    #[test]
    fn rewriting_a_function_on_a_d3_edge_is_critical_even_if_contract_is_fine() {
        let a = ShardActor::new(payments_module());
        let mut m = payments_module();
        // fee variant from contract tests
        let f = &mut m.functions[0];
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
        let r = a.propose(&m);
        assert!(r.refinement.is_refinement());
        assert_eq!(r.risk, Risk::Critical);
        assert!(matches!(r.decision, Decision::Reject(_)));
        // a permissive policy lets it through with a warning
        let mut lax = ShardActor::with_policy(
            payments_module(),
            SwapPolicy {
                reject_on_critical: false,
                ..Default::default()
            },
        );
        let r = lax.swap(m, "fee").unwrap();
        assert!(matches!(r.decision, Decision::AcceptWithWarnings(_)));
    }

    #[test]
    fn delegating_a_guard_to_a_pure_callee_is_accepted_with_medium_risk() {
        let mut a = ShardActor::new(payments_module());
        let r = a.swap(payments_module_v2(), "delegate guard").unwrap();
        assert_eq!(r.decision, Decision::Accept);
        assert_eq!(r.risk, Risk::Medium);
        assert!(r.affected.is_empty());
        // the new module has a control edge Transfer → CheckBalance
        let t = a.derived.node_of("Transfer").unwrap();
        let c = a.derived.node_of("CheckBalance").unwrap();
        assert_eq!(a.derived.graph.edges_between(t, c).len(), 2);
    }

    #[test]
    fn ownership_errors_block_the_swap() {
        let mut a = ShardActor::new(payments_module());
        let mut m = payments_module();
        m.functions[0] = transfer_use_after_consume();
        let r = a.swap(m, "buggy").unwrap_err();
        assert!(
            matches!(r.decision, Decision::Reject(ref why) if why.iter().any(|w| w.contains("ownership")))
        );
    }

    #[test]
    fn actor_executes_against_persistent_heap_across_swaps() {
        let mut a = ShardActor::new(payments_module());
        let x = a.heap.alloc(WALLET);
        let y = a.heap.alloc(WALLET);
        a.heap.set_field(x, BALANCE, Value::I64(100)).unwrap();
        a.heap.set_field(y, BALANCE, Value::I64(0)).unwrap();
        a.call(
            "Transfer",
            vec![Value::Obj(x), Value::Obj(y), Value::I64(40)],
        )
        .unwrap();
        a.swap(payments_module(), "same").unwrap();
        a.call(
            "Transfer",
            vec![Value::Obj(x), Value::Obj(y), Value::I64(10)],
        )
        .unwrap();
        assert_eq!(
            (a.heap.field_i64(x, BALANCE), a.heap.field_i64(y, BALANCE)),
            (Some(50), Some(50))
        );
    }
}
