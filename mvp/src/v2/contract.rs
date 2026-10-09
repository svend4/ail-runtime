//! Contracts and the refinement (non-weakening) check for hot-swap.
//!
//! v1's "weakening check" is: every old function *name* still exists. v2
//! compares *symbolic contracts* extracted from the code (`symbolic::effects`):
//!
//! | rule                                   | class      | rationale                                  |
//! |----------------------------------------|------------|--------------------------------------------|
//! | function removed                       | violation  | callers break                              |
//! | arity / parameter type changed         | violation  | callers break                              |
//! | guard (assert) dropped                 | violation  | the "weakening" the docs talk about        |
//! | new field written                      | violation  | effect footprint widened (new side effect)  |
//! | new field read                         | change     | wider precondition; review, not block      |
//! | guard added                            | change     | Liskov: stricter precondition can break callers |
//! | written expression changed             | change     | semantic change; drives connectivity risk  |
//! | return expression changed              | change     |                                            |
//! | callee set changed                     | change     | control edges move                         |
//! | function added                         | change     |                                            |

use super::ast::{Function, Module, ParamType};
use super::symbolic::{effects, effects_in, Effects, Sym};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    pub name: String,
    pub params: Vec<ParamType>,
    pub effects: Effects,
}

pub fn contract_of(f: &Function) -> Contract {
    Contract {
        name: f.name.clone(),
        params: f.params.clone(),
        effects: effects(f),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    Removed {
        name: String,
    },
    ArityChanged {
        name: String,
        old: usize,
        new: usize,
    },
    ParamTypeChanged {
        name: String,
        index: usize,
        old: ParamType,
        new: ParamType,
    },
    GuardDropped {
        name: String,
        guard: Sym,
    },
    NewWrite {
        name: String,
        target: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    Added {
        name: String,
    },
    NewRead {
        name: String,
        target: String,
    },
    GuardAdded {
        name: String,
        guard: Sym,
    },
    WriteChanged {
        name: String,
        target: String,
        old: Sym,
        new: Sym,
    },
    ReturnChanged {
        name: String,
        old: Option<Sym>,
        new: Option<Sym>,
    },
    CallsChanged {
        name: String,
        old: Vec<String>,
        new: Vec<String>,
    },
}

impl Change {
    /// A semantic change alters the post-state, the return value or the
    /// guards a caller observes. Structural changes (`Added`, `CallsChanged`)
    /// do not, and therefore do not by themselves put incident edges at risk.
    pub fn is_semantic(&self) -> bool {
        !matches!(self, Change::Added { .. } | Change::CallsChanged { .. })
    }

    pub fn function(&self) -> &str {
        match self {
            Change::Added { name }
            | Change::NewRead { name, .. }
            | Change::GuardAdded { name, .. }
            | Change::WriteChanged { name, .. }
            | Change::ReturnChanged { name, .. }
            | Change::CallsChanged { name, .. } => name,
        }
    }
}

impl Violation {
    pub fn function(&self) -> &str {
        match self {
            Violation::Removed { name }
            | Violation::ArityChanged { name, .. }
            | Violation::ParamTypeChanged { name, .. }
            | Violation::GuardDropped { name, .. }
            | Violation::NewWrite { name, .. } => name,
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Violation::Removed { name } => write!(f, "`{name}` removed"),
            Violation::ArityChanged { name, old, new } => write!(f, "`{name}` arity {old} → {new}"),
            Violation::ParamTypeChanged {
                name,
                index,
                old,
                new,
            } => write!(f, "`{name}` param {index}: {old:?} → {new:?}"),
            Violation::GuardDropped { name, guard } => write!(f, "`{name}` dropped guard {guard}"),
            Violation::NewWrite { name, target } => write!(f, "`{name}` now writes {target}"),
        }
    }
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Change::Added { name } => write!(f, "`{name}` added"),
            Change::NewRead { name, target } => write!(f, "`{name}` now reads {target}"),
            Change::GuardAdded { name, guard } => write!(f, "`{name}` added guard {guard}"),
            Change::WriteChanged {
                name,
                target,
                old,
                new,
            } => write!(f, "`{name}` {target} := {old}  ⇒  {new}"),
            Change::ReturnChanged { name, old, new } => {
                write!(f, "`{name}` return {} ⇒ {}", fmt_opt(old), fmt_opt(new))
            }
            Change::CallsChanged { name, old, new } => {
                write!(f, "`{name}` calls {old:?} ⇒ {new:?}")
            }
        }
    }
}

fn fmt_opt(s: &Option<Sym>) -> String {
    s.as_ref()
        .map(|x| x.to_string())
        .unwrap_or_else(|| "∅".into())
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Refinement {
    pub violations: Vec<Violation>,
    pub changes: Vec<Change>,
}

impl Refinement {
    /// `new` may replace `old` without weakening any contract.
    pub fn is_refinement(&self) -> bool {
        self.violations.is_empty()
    }
    /// Functions mentioned by any violation or change.
    pub fn touched_functions(&self) -> BTreeSet<String> {
        self.violations
            .iter()
            .map(|v| v.function().to_string())
            .chain(self.changes.iter().map(|c| c.function().to_string()))
            .collect()
    }

    /// Functions whose observable semantics changed (drives connectivity risk).
    pub fn semantically_touched(&self) -> BTreeSet<String> {
        self.violations
            .iter()
            .map(|v| v.function().to_string())
            .chain(
                self.changes
                    .iter()
                    .filter(|c| c.is_semantic())
                    .map(|c| c.function().to_string()),
            )
            .collect()
    }

    pub fn has_structural_changes(&self) -> bool {
        self.changes.iter().any(|c| !c.is_semantic())
    }
}

fn loc_str(l: &(Sym, u16)) -> String {
    format!("{}.f{}", l.0, l.1)
}

pub fn check_refinement(old: &Module, new: &Module) -> Refinement {
    let mut r = Refinement::default();
    for of in &old.functions {
        let name = of.name.clone();
        let Some(nf) = new.function(&of.name) else {
            r.violations.push(Violation::Removed { name });
            continue;
        };
        if of.params.len() != nf.params.len() {
            r.violations.push(Violation::ArityChanged {
                name,
                old: of.params.len(),
                new: nf.params.len(),
            });
            continue;
        }
        for (i, (a, b)) in of.params.iter().zip(&nf.params).enumerate() {
            if a != b {
                r.violations.push(Violation::ParamTypeChanged {
                    name: name.clone(),
                    index: i,
                    old: *a,
                    new: *b,
                });
            }
        }
        let oe = effects_in(of, old);
        let ne = effects_in(nf, new);
        for g in oe.guards.difference(&ne.guards) {
            r.violations.push(Violation::GuardDropped {
                name: name.clone(),
                guard: g.clone(),
            });
        }
        for g in ne.guards.difference(&oe.guards) {
            r.changes.push(Change::GuardAdded {
                name: name.clone(),
                guard: g.clone(),
            });
        }
        for (loc, v) in &ne.writes {
            match oe.writes.get(loc) {
                None => r.violations.push(Violation::NewWrite {
                    name: name.clone(),
                    target: loc_str(loc),
                }),
                Some(ov) if ov != v => r.changes.push(Change::WriteChanged {
                    name: name.clone(),
                    target: loc_str(loc),
                    old: ov.clone(),
                    new: v.clone(),
                }),
                _ => {}
            }
        }
        for loc in &oe.writes {
            if !ne.writes.contains_key(loc.0) {
                // dropping a write is a semantic change (post-state differs)
                r.changes.push(Change::WriteChanged {
                    name: name.clone(),
                    target: loc_str(loc.0),
                    old: loc.1.clone(),
                    new: Sym::Unknown,
                });
            }
        }
        for loc in ne.reads.difference(&oe.reads) {
            r.changes.push(Change::NewRead {
                name: name.clone(),
                target: loc_str(loc),
            });
        }
        if oe.ret != ne.ret {
            r.changes.push(Change::ReturnChanged {
                name: name.clone(),
                old: oe.ret.clone(),
                new: ne.ret.clone(),
            });
        }
        if oe.calls != ne.calls {
            r.changes.push(Change::CallsChanged {
                name: name.clone(),
                old: oe.calls.iter().cloned().collect(),
                new: ne.calls.iter().cloned().collect(),
            });
        }
    }
    for nf in &new.functions {
        if old.function(&nf.name).is_none() {
            r.changes.push(Change::Added {
                name: nf.name.clone(),
            });
        }
    }
    r
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::super::ast::*;
    use super::*;

    #[test]
    fn identical_module_is_a_refinement_with_no_changes() {
        let r = check_refinement(&payments_module(), &payments_module());
        assert!(r.is_refinement());
        assert!(r.changes.is_empty());
    }

    #[test]
    fn removing_a_function_is_a_violation() {
        let mut m = payments_module();
        m.functions.retain(|f| f.name != "Refund");
        let r = check_refinement(&payments_module(), &m);
        assert_eq!(
            r.violations,
            vec![Violation::Removed {
                name: "Refund".into()
            }]
        );
    }

    #[test]
    fn dropping_the_balance_guard_is_a_violation_even_though_the_name_survives() {
        let mut m = payments_module();
        m.functions[0] = transfer_without_guard();
        let r = check_refinement(&payments_module(), &m);
        assert!(!r.is_refinement());
        assert!(
            matches!(&r.violations[0], Violation::GuardDropped { name, .. } if name == "Transfer")
        );
    }

    #[test]
    fn delegating_the_guard_to_a_pure_call_is_a_refinement() {
        // Transfer v2 replaces the inline guard by `CheckBalance(...)`; the
        // callee is pure, so its return expression is inlined and the guards
        // compare equal. Only the call structure changed.
        let r = check_refinement(&payments_module(), &payments_module_v2());
        assert!(r.is_refinement(), "{:?}", r.violations);
        assert_eq!(r.changes.len(), 1);
        assert!(matches!(&r.changes[0], Change::CallsChanged { .. }));
        assert!(r.semantically_touched().is_empty());
        assert!(r.has_structural_changes());
        // and the reverse direction (inlining a pure call back) is also fine
        assert!(check_refinement(&payments_module_v2(), &payments_module()).is_refinement());
    }

    #[test]
    fn new_side_effect_is_a_violation() {
        let mut m = payments_module();
        // Transfer now also writes field 1 of `to`
        m.functions[0].body.insert(
            8,
            Op::StoreField {
                obj: Reg(1),
                field: 1,
                value: Reg(2),
            },
        );
        let r = check_refinement(&payments_module(), &m);
        assert!(
            matches!(&r.violations[0], Violation::NewWrite { target, .. } if target == "p1.f1")
        );
    }

    #[test]
    fn changed_arithmetic_is_a_change_not_a_violation() {
        let mut m = payments_module();
        // fee: to receives amount - 1
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
        // AddChecked now uses Reg(10)
        if let Op::AddChecked { b, .. } = &mut f.body[8] {
            *b = Reg(10);
        }
        let r = check_refinement(&payments_module(), &m);
        assert!(r.is_refinement(), "{:?}", r.violations);
        assert!(r
            .changes
            .iter()
            .any(|c| matches!(c, Change::WriteChanged { target, .. } if target == "p1.f0")));
    }
}
