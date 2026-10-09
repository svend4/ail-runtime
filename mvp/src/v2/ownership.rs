//! Static ownership analyzer over a function body.
//!
//! `docs/ownership-analyzer.md` describes this component; v1 never
//! implemented it (the interpreter only catches `UseAfterConsume` at run
//! time). This pass proves, before a module is accepted, that:
//!
//! * no register is read before it is initialised,
//! * no register is read after it was moved out or consumed,
//! * nothing is consumed twice,
//! * register indices stay inside the declared register file,
//! * a value flowing out of `Return` is live,
//! * two *object* parameters that the function mutates must be distinct
//!   (the `&mut` + `&mut` rule). This cannot be proven statically, so it is
//!   reported as an obligation the VM checks on every call (`vm::Vm`).

use super::ast::{Function, Op, ParamType, Reg};
use super::symbolic::{effects, Sym};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegState {
    Uninit,
    Owned,
    Consumed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnershipError {
    UseBeforeInit {
        op: usize,
        reg: Reg,
    },
    UseAfterConsume {
        op: usize,
        reg: Reg,
    },
    DoubleConsume {
        op: usize,
        reg: Reg,
    },
    InvalidReg {
        op: usize,
        reg: Reg,
    },
    ReturnConsumed {
        op: usize,
        reg: Reg,
    },
    /// Body falls off the end without `Return`.
    MissingReturn,
    /// Code after `Return` can never run.
    UnreachableAfterReturn {
        op: usize,
    },
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnershipError::UseBeforeInit { op, reg } => {
                write!(f, "op#{op}: r{} read before initialisation", reg.0)
            }
            OwnershipError::UseAfterConsume { op, reg } => {
                write!(f, "op#{op}: r{} used after move/consume", reg.0)
            }
            OwnershipError::DoubleConsume { op, reg } => {
                write!(f, "op#{op}: r{} consumed twice", reg.0)
            }
            OwnershipError::InvalidReg { op, reg } => {
                write!(f, "op#{op}: r{} outside register file", reg.0)
            }
            OwnershipError::ReturnConsumed { op, reg } => {
                write!(f, "op#{op}: returning consumed r{}", reg.0)
            }
            OwnershipError::MissingReturn => write!(f, "body ends without Return"),
            OwnershipError::UnreachableAfterReturn { op } => {
                write!(f, "op#{op}: unreachable after Return")
            }
        }
    }
}

/// A proof obligation that must be discharged at call time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Obligation {
    /// Parameters `a` and `b` are both mutated object parameters of the same
    /// type: the caller must pass distinct objects.
    DistinctMutableParams { a: u16, b: u16 },
}

impl fmt::Display for Obligation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Obligation::DistinctMutableParams { a, b } => {
                write!(f, "p{a} and p{b} are both mutated: must not alias")
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    pub errors: Vec<OwnershipError>,
    pub obligations: Vec<Obligation>,
    /// Registers written but never read (dead stores) — hints, not errors.
    pub dead_stores: Vec<(usize, Reg)>,
}

impl Report {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

pub fn analyze(f: &Function) -> Report {
    let mut rep = Report::default();
    let n = f.regs as usize;
    let mut st = vec![RegState::Uninit; n];
    for slot in st.iter_mut().take(f.params.len()) {
        *slot = RegState::Owned;
    }
    if f.params.len() > n {
        for i in n..f.params.len() {
            rep.errors.push(OwnershipError::InvalidReg {
                op: 0,
                reg: Reg(i as u16),
            });
        }
    }

    let mut last_write: Vec<Option<usize>> = vec![None; n];
    let mut returned = false;

    for (i, op) in f.body.iter().enumerate() {
        if returned {
            rep.errors
                .push(OwnershipError::UnreachableAfterReturn { op: i });
            break;
        }
        // reads
        for r in op.reads() {
            let idx = r.0 as usize;
            if idx >= n {
                rep.errors
                    .push(OwnershipError::InvalidReg { op: i, reg: r });
                continue;
            }
            match st[idx] {
                RegState::Uninit => rep
                    .errors
                    .push(OwnershipError::UseBeforeInit { op: i, reg: r }),
                RegState::Consumed => match op {
                    Op::Consume { .. } => rep
                        .errors
                        .push(OwnershipError::DoubleConsume { op: i, reg: r }),
                    Op::Return { .. } => rep
                        .errors
                        .push(OwnershipError::ReturnConsumed { op: i, reg: r }),
                    _ => rep
                        .errors
                        .push(OwnershipError::UseAfterConsume { op: i, reg: r }),
                },
                RegState::Owned => {}
            }
            last_write[idx] = None; // the value was used
        }
        // linear transitions
        match op {
            Op::Move { src, .. } => {
                if let Some(s) = st.get_mut(src.0 as usize) {
                    *s = RegState::Consumed;
                }
            }
            Op::Consume { reg } => {
                if let Some(s) = st.get_mut(reg.0 as usize) {
                    *s = RegState::Consumed;
                }
            }
            Op::Return { .. } => returned = true,
            _ => {}
        }
        // writes
        if let Some(w) = op.writes() {
            let idx = w.0 as usize;
            if idx >= n {
                rep.errors
                    .push(OwnershipError::InvalidReg { op: i, reg: w });
            } else {
                if let Some(prev) = last_write[idx] {
                    rep.dead_stores.push((prev, w));
                }
                st[idx] = RegState::Owned;
                last_write[idx] = Some(i);
            }
        }
    }
    if !returned {
        rep.errors.push(OwnershipError::MissingReturn);
    }

    // aliasing obligations: mutated object params of the same type
    let eff = effects(f);
    let mut mutated: BTreeSet<u16> = BTreeSet::new();
    for (obj, _) in eff.writes.keys() {
        if let Sym::Param(i) = obj {
            mutated.insert(*i);
        }
    }
    let m: Vec<u16> = mutated.into_iter().collect();
    for (ai, a) in m.iter().enumerate() {
        for b in m.iter().skip(ai + 1) {
            let same_type = matches!(
                (f.params.get(*a as usize), f.params.get(*b as usize)),
                (Some(ParamType::Object(x)), Some(ParamType::Object(y))) if x == y
            );
            if same_type {
                rep.obligations
                    .push(Obligation::DistinctMutableParams { a: *a, b: *b });
            }
        }
    }
    rep
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::super::ast::*;
    use super::*;

    #[test]
    fn transfer_is_clean_but_carries_an_alias_obligation() {
        let r = analyze(&transfer());
        assert!(r.is_ok(), "{:?}", r.errors);
        assert_eq!(
            r.obligations,
            vec![Obligation::DistinctMutableParams { a: 0, b: 1 }]
        );
    }

    #[test]
    fn detects_use_after_consume_statically() {
        let r = analyze(&transfer_use_after_consume());
        assert!(
            r.errors
                .iter()
                .any(|e| matches!(e, OwnershipError::UseAfterConsume { reg: Reg(2), .. })),
            "{:?}",
            r.errors
        );
    }

    #[test]
    fn detects_uninitialised_and_out_of_range() {
        let f = Function {
            name: "Bad".into(),
            params: vec![ParamType::I64],
            regs: 2,
            body: vec![
                Op::AddChecked {
                    a: Reg(0),
                    b: Reg(1),
                    dest: Reg(1),
                },
                Op::Move {
                    src: Reg(1),
                    dest: Reg(7),
                },
                Op::Return {
                    value: Some(Reg(1)),
                },
            ],
        };
        let r = analyze(&f);
        assert!(r
            .errors
            .contains(&OwnershipError::UseBeforeInit { op: 0, reg: Reg(1) }));
        assert!(r
            .errors
            .contains(&OwnershipError::InvalidReg { op: 1, reg: Reg(7) }));
        assert!(r
            .errors
            .contains(&OwnershipError::ReturnConsumed { op: 2, reg: Reg(1) }));
    }

    #[test]
    fn missing_return_and_dead_store() {
        let f = Function {
            name: "NoRet".into(),
            params: vec![],
            regs: 1,
            body: vec![
                Op::ConstI64 {
                    value: 1,
                    dest: Reg(0),
                },
                Op::ConstI64 {
                    value: 2,
                    dest: Reg(0),
                },
            ],
        };
        let r = analyze(&f);
        assert!(r.errors.contains(&OwnershipError::MissingReturn));
        assert_eq!(r.dead_stores, vec![(0, Reg(0))]);
    }

    #[test]
    fn read_only_function_has_no_obligation() {
        assert!(analyze(&check_balance()).obligations.is_empty());
    }
}
