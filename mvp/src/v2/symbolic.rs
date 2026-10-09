//! Symbolic evaluation of a function body.
//!
//! Straight-line Binary AST is evaluated over *symbolic* values so that a
//! function's contract can be read off its code: which fields it reads
//! (pre-state), which it writes and with what expression (post-state), which
//! conditions it asserts (guards) and whom it calls. Register names drop out,
//! so two compilations of the same `.ail` source produce identical effects.

use super::ast::{Function, Module, Op, ParamType, Reg, TypeTag};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Symbolic value.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sym {
    Param(u16),
    Const(i64),
    ConstBool(bool),
    /// Pre-state field value: `obj.field` as it was on entry.
    Field(Box<Sym>, u16),
    Add(Box<Sym>, Box<Sym>),
    Sub(Box<Sym>, Box<Sym>),
    Ge(Box<Sym>, Box<Sym>),
    Call(String, Vec<Sym>),
    Unknown,
}

impl fmt::Display for Sym {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Sym::Param(i) => write!(f, "p{i}"),
            Sym::Const(c) => write!(f, "{c}"),
            Sym::ConstBool(b) => write!(f, "{b}"),
            Sym::Field(o, fld) => write!(f, "{o}.f{fld}"),
            Sym::Add(a, b) => write!(f, "({a} + {b})"),
            Sym::Sub(a, b) => write!(f, "({a} - {b})"),
            Sym::Ge(a, b) => write!(f, "({a} >= {b})"),
            Sym::Call(n, args) => {
                write!(f, "{n}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{a}")?;
                }
                write!(f, ")")
            }
            Sym::Unknown => write!(f, "?"),
        }
    }
}

/// A field location: which object expression, which field.
pub type Loc = (Sym, u16);

/// Everything the function does, independent of register allocation.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Effects {
    /// Fields read from the pre-state.
    pub reads: BTreeSet<Loc>,
    /// Final symbolic value stored into each written field.
    pub writes: BTreeMap<Loc, Sym>,
    /// Asserted conditions (in symbolic form).
    pub guards: BTreeSet<Sym>,
    /// Callees.
    pub calls: BTreeSet<String>,
    /// Symbolic return value.
    pub ret: Option<Sym>,
    /// Footprint by nominal type: `(type, field)` read / written. This is
    /// what lets two functions over *different* parameters be related.
    pub typed_reads: BTreeSet<(TypeTag, u16)>,
    pub typed_writes: BTreeSet<(TypeTag, u16)>,
}

impl Effects {
    pub fn footprint(&self) -> BTreeSet<(TypeTag, u16)> {
        self.typed_reads
            .union(&self.typed_writes)
            .cloned()
            .collect()
    }
    pub fn is_pure(&self) -> bool {
        self.writes.is_empty()
    }
}

fn param_type_of(f: &Function, s: &Sym) -> Option<TypeTag> {
    match s {
        Sym::Param(i) => match f.params.get(*i as usize) {
            Some(ParamType::Object(t)) => Some(*t),
            _ => None,
        },
        _ => None,
    }
}

/// Symbolically execute `f`. Unknown or ill-formed reads yield `Sym::Unknown`
/// rather than failing: the ownership analyzer is the place that reports
/// register misuse, this pass only extracts meaning.
pub fn effects(f: &Function) -> Effects {
    let mut regs: BTreeMap<Reg, Sym> = BTreeMap::new();
    for (i, _) in f.params.iter().enumerate() {
        regs.insert(Reg(i as u16), Sym::Param(i as u16));
    }
    let mut store: BTreeMap<Loc, Sym> = BTreeMap::new();
    let mut eff = Effects::default();

    let get = |regs: &BTreeMap<Reg, Sym>, r: Reg| regs.get(&r).cloned().unwrap_or(Sym::Unknown);

    for op in &f.body {
        match op {
            Op::ConstI64 { value, dest } => {
                regs.insert(*dest, Sym::Const(*value));
            }
            Op::ConstBool { value, dest } => {
                regs.insert(*dest, Sym::ConstBool(*value));
            }
            Op::Move { src, dest } => {
                let v = get(&regs, *src);
                regs.remove(src);
                regs.insert(*dest, v);
            }
            Op::Consume { reg } => {
                regs.remove(reg);
            }
            Op::AddChecked { a, b, dest } => {
                let v = Sym::Add(Box::new(get(&regs, *a)), Box::new(get(&regs, *b)));
                regs.insert(*dest, v);
            }
            Op::SubChecked { a, b, dest } => {
                let v = Sym::Sub(Box::new(get(&regs, *a)), Box::new(get(&regs, *b)));
                regs.insert(*dest, v);
            }
            Op::Ge { a, b, dest } => {
                let v = Sym::Ge(Box::new(get(&regs, *a)), Box::new(get(&regs, *b)));
                regs.insert(*dest, v);
            }
            Op::LoadField { obj, field, dest } => {
                let o = get(&regs, *obj);
                let loc = (o.clone(), *field);
                let v = if let Some(written) = store.get(&loc) {
                    written.clone()
                } else {
                    eff.reads.insert(loc.clone());
                    if let Some(t) = param_type_of(f, &o) {
                        eff.typed_reads.insert((t, *field));
                    }
                    Sym::Field(Box::new(o), *field)
                };
                regs.insert(*dest, v);
            }
            Op::StoreField { obj, field, value } => {
                let o = get(&regs, *obj);
                let v = get(&regs, *value);
                if let Some(t) = param_type_of(f, &o) {
                    eff.typed_writes.insert((t, *field));
                }
                store.insert((o, *field), v);
            }
            Op::Assert { cond, .. } => {
                eff.guards.insert(get(&regs, *cond));
            }
            Op::Call { func, args, dest } => {
                let a: Vec<Sym> = args.iter().map(|r| get(&regs, *r)).collect();
                eff.calls.insert(func.clone());
                if let Some(d) = dest {
                    regs.insert(*d, Sym::Call(func.clone(), a));
                }
            }
            Op::Return { value } => {
                eff.ret = Some(value.map(|r| get(&regs, r)).unwrap_or(Sym::ConstBool(true)));
                break;
            }
        }
    }
    eff.writes = store;
    eff
}

// ---------------------------------------------------------------------------
// Inter-procedural normalisation: inline pure callees
// ---------------------------------------------------------------------------

/// Substitute parameters inside `sym` with the given argument expressions.
pub fn subst(sym: &Sym, args: &[Sym]) -> Sym {
    match sym {
        Sym::Param(i) => args.get(*i as usize).cloned().unwrap_or(Sym::Unknown),
        Sym::Field(o, f) => Sym::Field(Box::new(subst(o, args)), *f),
        Sym::Add(a, b) => Sym::Add(Box::new(subst(a, args)), Box::new(subst(b, args))),
        Sym::Sub(a, b) => Sym::Sub(Box::new(subst(a, args)), Box::new(subst(b, args))),
        Sym::Ge(a, b) => Sym::Ge(Box::new(subst(a, args)), Box::new(subst(b, args))),
        Sym::Call(n, xs) => Sym::Call(n.clone(), xs.iter().map(|x| subst(x, args)).collect()),
        other => other.clone(),
    }
}

/// Replace `Call(f, args)` by `f`'s symbolic return value whenever `f` is a
/// *pure* function of `m` (no writes). Bounded by `depth` to stay total on
/// recursive modules. Everything else is left untouched, so a call to an
/// impure or unknown function stays an opaque `Call`.
pub fn inline_pure_calls(sym: &Sym, m: &Module, depth: usize) -> Sym {
    match sym {
        Sym::Call(name, args) if depth > 0 => {
            let args: Vec<Sym> = args
                .iter()
                .map(|a| inline_pure_calls(a, m, depth))
                .collect();
            if let Some(f) = m.function(name) {
                let e = effects(f);
                if e.is_pure() {
                    if let Some(ret) = &e.ret {
                        return inline_pure_calls(&subst(ret, &args), m, depth - 1);
                    }
                }
            }
            Sym::Call(name.clone(), args)
        }
        Sym::Field(o, f) => Sym::Field(Box::new(inline_pure_calls(o, m, depth)), *f),
        Sym::Add(a, b) => Sym::Add(
            Box::new(inline_pure_calls(a, m, depth)),
            Box::new(inline_pure_calls(b, m, depth)),
        ),
        Sym::Sub(a, b) => Sym::Sub(
            Box::new(inline_pure_calls(a, m, depth)),
            Box::new(inline_pure_calls(b, m, depth)),
        ),
        Sym::Ge(a, b) => Sym::Ge(
            Box::new(inline_pure_calls(a, m, depth)),
            Box::new(inline_pure_calls(b, m, depth)),
        ),
        other => other.clone(),
    }
}

/// Effects of `f` normalised against its module: pure callees are inlined in
/// guards, written values and the return value, so `assert CheckBalance(w, a)`
/// and `assert w.balance >= a` compare equal. Reads contributed by inlined
/// pure callees are added to the footprint.
pub fn effects_in(f: &Function, m: &Module) -> Effects {
    const DEPTH: usize = 8;
    let mut e = effects(f);
    e.guards = e
        .guards
        .iter()
        .map(|g| inline_pure_calls(g, m, DEPTH))
        .collect();
    e.writes = e
        .writes
        .iter()
        .map(|(k, v)| (k.clone(), inline_pure_calls(v, m, DEPTH)))
        .collect();
    e.ret = e.ret.as_ref().map(|r| inline_pure_calls(r, m, DEPTH));
    // footprint of inlined pure callees (reads only, by construction)
    let mut stack: Vec<String> = e.calls.iter().cloned().collect();
    let mut seen = BTreeSet::new();
    while let Some(name) = stack.pop() {
        if !seen.insert(name.clone()) {
            continue;
        }
        if let Some(g) = m.function(&name) {
            let ge = effects(g);
            if ge.is_pure() {
                e.typed_reads.extend(ge.typed_reads.iter().cloned());
                stack.extend(ge.calls.iter().cloned());
            }
        }
    }
    e
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::*;

    #[test]
    fn transfer_contract_is_read_off_the_code() {
        let e = effects(&transfer());
        let p0 = Sym::Param(0);
        let p1 = Sym::Param(1);
        let p2 = Sym::Param(2);
        let bal0 = Sym::Field(Box::new(p0.clone()), BALANCE);
        let bal1 = Sym::Field(Box::new(p1.clone()), BALANCE);
        assert_eq!(e.reads.len(), 2);
        assert_eq!(
            e.writes[&(p0.clone(), BALANCE)],
            Sym::Sub(Box::new(bal0.clone()), Box::new(p2.clone()))
        );
        assert_eq!(
            e.writes[&(p1.clone(), BALANCE)],
            Sym::Add(Box::new(bal1.clone()), Box::new(p2.clone()))
        );
        assert_eq!(e.guards.len(), 1);
        assert!(e.guards.contains(&Sym::Ge(Box::new(bal0), Box::new(p2))));
        assert_eq!(e.ret, Some(Sym::ConstBool(true)));
        assert_eq!(e.typed_writes.len(), 1); // (WALLET, BALANCE)
    }

    #[test]
    fn register_renaming_does_not_change_effects() {
        let mut g = transfer();
        // shift every temp register by 10
        for op in &mut g.body {
            let bump = |r: &mut Reg| {
                if r.0 >= 3 {
                    r.0 += 10
                }
            };
            match op {
                Op::LoadField { dest, .. }
                | Op::Ge { dest, .. }
                | Op::SubChecked { dest, .. }
                | Op::AddChecked { dest, .. }
                | Op::ConstBool { dest, .. } => bump(dest),
                _ => {}
            }
            match op {
                Op::Ge { a, b, .. } | Op::SubChecked { a, b, .. } | Op::AddChecked { a, b, .. } => {
                    bump(a);
                    bump(b);
                }
                Op::StoreField { value, .. } => bump(value),
                Op::Assert { cond, .. } => bump(cond),
                Op::Return { value: Some(v) } => bump(v),
                _ => {}
            }
        }
        g.regs = 20;
        assert_ne!(g.body, transfer().body);
        assert_eq!(effects(&g), effects(&transfer()));
    }

    #[test]
    fn dropped_guard_is_visible() {
        assert!(effects(&transfer_without_guard()).guards.is_empty());
    }

    #[test]
    fn call_edges_are_captured() {
        let e = effects(&transfer_via_check());
        assert!(e.calls.contains("CheckBalance"));
        assert!(matches!(e.guards.iter().next(), Some(Sym::Call(n, _)) if n == "CheckBalance"));
    }

    #[test]
    fn pure_call_inlining_makes_delegated_guard_equal_to_inline_guard() {
        let m = payments_module_v2();
        let e1 = effects_in(m.function("Transfer").unwrap(), &m);
        let e0 = effects_in(transfer().clone().as_ref(), &m);
        assert_eq!(e1.guards, e0.guards, "{:?} vs {:?}", e1.guards, e0.guards);
        assert_eq!(e1.writes, e0.writes);
        assert!(e1.typed_reads.contains(&(WALLET, BALANCE)));
        // impure callee stays opaque
        let m2 = Module {
            name: "M".into(),
            functions: vec![transfer_via_check(), transfer_renamed("CheckBalance")],
        };
        let e2 = effects_in(m2.function("Transfer").unwrap(), &m2);
        assert!(matches!(e2.guards.iter().next(), Some(Sym::Call(..))));
    }

    #[test]
    fn display_is_readable() {
        let e = effects(&transfer());
        let w = e.writes.values().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(w.contains(&"(p0.f0 - p2)".to_string()));
    }
}
