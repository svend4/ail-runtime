//! Binary AST v2 — the executable graph of operations, now in the library.
//!
//! In v1 the AST lives in `main.rs` (binary crate), so no analyzer in the
//! library can see it; that is the root cause of the hand-declared relation
//! graph. v2 moves the AST into the library, adds parameter *types* (so two
//! functions touching `Wallet.balance` can be related), and adds `Call` so
//! control-flow edges between functions exist in the code itself.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Reg(pub u16);

/// Nominal object type (e.g. `Wallet`). Fields are `u16` ids inside a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TypeTag(pub u16);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ParamType {
    I64,
    Bool,
    Object(TypeTag),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Op {
    ConstI64 {
        value: i64,
        dest: Reg,
    },
    ConstBool {
        value: bool,
        dest: Reg,
    },
    /// Linear move: `src` becomes `Consumed`.
    Move {
        src: Reg,
        dest: Reg,
    },
    Consume {
        reg: Reg,
    },
    AddChecked {
        a: Reg,
        b: Reg,
        dest: Reg,
    },
    SubChecked {
        a: Reg,
        b: Reg,
        dest: Reg,
    },
    Ge {
        a: Reg,
        b: Reg,
        dest: Reg,
    },
    LoadField {
        obj: Reg,
        field: u16,
        dest: Reg,
    },
    StoreField {
        obj: Reg,
        field: u16,
        value: Reg,
    },
    Assert {
        cond: Reg,
        error_code: u16,
    },
    /// Call another function of the same module by name.
    Call {
        func: String,
        args: Vec<Reg>,
        dest: Option<Reg>,
    },
    Return {
        value: Option<Reg>,
    },
}

impl Op {
    /// Registers read by this op (in order).
    pub fn reads(&self) -> Vec<Reg> {
        match self {
            Op::ConstI64 { .. } | Op::ConstBool { .. } => vec![],
            Op::Move { src, .. } => vec![*src],
            Op::Consume { reg } => vec![*reg],
            Op::AddChecked { a, b, .. } | Op::SubChecked { a, b, .. } | Op::Ge { a, b, .. } => {
                vec![*a, *b]
            }
            Op::LoadField { obj, .. } => vec![*obj],
            Op::StoreField { obj, value, .. } => vec![*obj, *value],
            Op::Assert { cond, .. } => vec![*cond],
            Op::Call { args, .. } => args.clone(),
            Op::Return { value } => value.iter().copied().collect(),
        }
    }

    /// Register written by this op, if any.
    pub fn writes(&self) -> Option<Reg> {
        match self {
            Op::ConstI64 { dest, .. }
            | Op::ConstBool { dest, .. }
            | Op::Move { dest, .. }
            | Op::AddChecked { dest, .. }
            | Op::SubChecked { dest, .. }
            | Op::Ge { dest, .. }
            | Op::LoadField { dest, .. } => Some(*dest),
            Op::Call { dest, .. } => *dest,
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<ParamType>,
    /// Size of the register file this function needs.
    pub regs: u16,
    pub body: Vec<Op>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub functions: Vec<Function>,
}

impl AsRef<Function> for Function {
    fn as_ref(&self) -> &Function {
        self
    }
}

impl Module {
    pub fn function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)
    }
    pub fn names(&self) -> Vec<&str> {
        self.functions.iter().map(|f| f.name.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Example module (the Payments domain used throughout the docs)
// ---------------------------------------------------------------------------

pub mod examples {
    use super::*;

    pub const WALLET: TypeTag = TypeTag(1);
    pub const BALANCE: u16 = 0;
    pub const ERR_INSUFFICIENT: u16 = 401;

    /// `Transfer(from: Wallet, to: Wallet, amount: i64) -> bool` compiled in
    /// the order written in `examples/compilation.md` (both loads first).
    pub fn transfer() -> Function {
        Function {
            name: "Transfer".into(),
            params: vec![
                ParamType::Object(WALLET),
                ParamType::Object(WALLET),
                ParamType::I64,
            ],
            regs: 9,
            body: vec![
                Op::LoadField {
                    obj: Reg(0),
                    field: BALANCE,
                    dest: Reg(3),
                },
                Op::LoadField {
                    obj: Reg(1),
                    field: BALANCE,
                    dest: Reg(4),
                },
                Op::Ge {
                    a: Reg(3),
                    b: Reg(2),
                    dest: Reg(5),
                },
                Op::Assert {
                    cond: Reg(5),
                    error_code: ERR_INSUFFICIENT,
                },
                Op::SubChecked {
                    a: Reg(3),
                    b: Reg(2),
                    dest: Reg(6),
                },
                Op::StoreField {
                    obj: Reg(0),
                    field: BALANCE,
                    value: Reg(6),
                },
                Op::AddChecked {
                    a: Reg(4),
                    b: Reg(2),
                    dest: Reg(7),
                },
                Op::StoreField {
                    obj: Reg(1),
                    field: BALANCE,
                    value: Reg(7),
                },
                Op::ConstBool {
                    value: true,
                    dest: Reg(8),
                },
                Op::Return {
                    value: Some(Reg(8)),
                },
            ],
        }
    }

    /// Same body as Transfer, different name — exactly as in the docs.
    pub fn refund() -> Function {
        let mut f = transfer();
        f.name = "Refund".into();
        f
    }

    /// `CheckBalance(w: Wallet, amount: i64) -> bool` — read-only guard.
    pub fn check_balance() -> Function {
        Function {
            name: "CheckBalance".into(),
            params: vec![ParamType::Object(WALLET), ParamType::I64],
            regs: 4,
            body: vec![
                Op::LoadField {
                    obj: Reg(0),
                    field: BALANCE,
                    dest: Reg(2),
                },
                Op::Ge {
                    a: Reg(2),
                    b: Reg(1),
                    dest: Reg(3),
                },
                Op::Return {
                    value: Some(Reg(3)),
                },
            ],
        }
    }

    /// Transfer that delegates its guard to `CheckBalance` (creates a control edge).
    pub fn transfer_via_check() -> Function {
        Function {
            name: "Transfer".into(),
            params: vec![
                ParamType::Object(WALLET),
                ParamType::Object(WALLET),
                ParamType::I64,
            ],
            regs: 10,
            body: vec![
                Op::Call {
                    func: "CheckBalance".into(),
                    args: vec![Reg(0), Reg(2)],
                    dest: Some(Reg(5)),
                },
                Op::Assert {
                    cond: Reg(5),
                    error_code: ERR_INSUFFICIENT,
                },
                Op::LoadField {
                    obj: Reg(0),
                    field: BALANCE,
                    dest: Reg(3),
                },
                Op::LoadField {
                    obj: Reg(1),
                    field: BALANCE,
                    dest: Reg(4),
                },
                Op::SubChecked {
                    a: Reg(3),
                    b: Reg(2),
                    dest: Reg(6),
                },
                Op::StoreField {
                    obj: Reg(0),
                    field: BALANCE,
                    value: Reg(6),
                },
                Op::AddChecked {
                    a: Reg(4),
                    b: Reg(2),
                    dest: Reg(7),
                },
                Op::StoreField {
                    obj: Reg(1),
                    field: BALANCE,
                    value: Reg(7),
                },
                Op::ConstBool {
                    value: true,
                    dest: Reg(8),
                },
                Op::Return {
                    value: Some(Reg(8)),
                },
            ],
        }
    }

    /// A *weakened* Transfer: the balance guard was dropped.
    pub fn transfer_without_guard() -> Function {
        let mut f = transfer();
        f.body
            .retain(|op| !matches!(op, Op::Ge { .. } | Op::Assert { .. }));
        f
    }

    /// Transfer with a use-after-consume bug (for the ownership analyzer).
    pub fn transfer_use_after_consume() -> Function {
        let mut f = transfer();
        // consume the amount register before the second arithmetic use
        f.body.insert(5, Op::Consume { reg: Reg(2) });
        f
    }

    /// Transfer's body under another name (an *impure* stand-in for tests).
    pub fn transfer_renamed(name: &str) -> Function {
        let mut f = transfer();
        f.name = name.into();
        f
    }

    pub fn payments_module() -> Module {
        Module {
            name: "PaymentsModule".into(),
            functions: vec![transfer(), refund(), check_balance()],
        }
    }

    pub fn payments_module_v2() -> Module {
        Module {
            name: "PaymentsModule".into(),
            functions: vec![transfer_via_check(), refund(), check_balance()],
        }
    }
}
