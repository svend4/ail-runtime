//! Interpreter v2 — register VM with calls, typed parameters, fuel and the
//! runtime half of the ownership rules.
//!
//! The v1 interpreter clones `ObjectRef`s freely: `Transfer(w, w, 300)` is
//! accepted and, with the compilation order given in `examples/compilation.md`
//! (both loads first), *creates* 300 units out of nothing. v2 discharges the
//! `DistinctMutableParams` obligations produced by `ownership::analyze` at
//! every call, so aliasing two mutable object parameters is an error.

use super::ast::{Function, Module, Op, ParamType, Reg, TypeTag};
use super::ownership::{analyze, Obligation};
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    I64(i64),
    Bool(bool),
    Obj(ObjectId),
    Unit,
    Consumed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Object {
    pub type_tag: TypeTag,
    pub fields: BTreeMap<u16, Value>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Heap {
    objects: BTreeMap<ObjectId, Object>,
    next: u64,
}

impl Heap {
    pub fn alloc(&mut self, type_tag: TypeTag) -> ObjectId {
        self.next += 1;
        let id = ObjectId(self.next);
        self.objects.insert(
            id,
            Object {
                type_tag,
                fields: BTreeMap::new(),
            },
        );
        id
    }
    pub fn get(&self, id: ObjectId) -> Option<&Object> {
        self.objects.get(&id)
    }
    pub fn set_field(&mut self, id: ObjectId, field: u16, v: Value) -> Result<(), VmError> {
        self.objects
            .get_mut(&id)
            .ok_or(VmError::ObjectNotFound(id))?
            .fields
            .insert(field, v);
        Ok(())
    }
    pub fn field(&self, id: ObjectId, field: u16) -> Result<&Value, VmError> {
        self.get(id)
            .ok_or(VmError::ObjectNotFound(id))?
            .fields
            .get(&field)
            .ok_or(VmError::FieldNotFound { obj: id, field })
    }
    pub fn field_i64(&self, id: ObjectId, field: u16) -> Option<i64> {
        match self.field(id, field) {
            Ok(Value::I64(x)) => Some(*x),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VmError {
    FunctionNotFound(String),
    ArityMismatch {
        func: String,
        expected: usize,
        found: usize,
    },
    ParamTypeMismatch {
        func: String,
        index: usize,
        expected: ParamType,
    },
    /// Two mutable object parameters received the same object.
    AliasedUniqueParams {
        func: String,
        a: u16,
        b: u16,
        object: ObjectId,
    },
    InvalidReg(Reg),
    TypeMismatch {
        op: usize,
    },
    Overflow {
        op: usize,
    },
    AssertFailed {
        code: u16,
    },
    UseAfterConsume {
        op: usize,
        reg: Reg,
    },
    ObjectNotFound(ObjectId),
    FieldNotFound {
        obj: ObjectId,
        field: u16,
    },
    CallDepthExceeded(usize),
    OutOfFuel,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

pub struct Vm<'m> {
    module: &'m Module,
    pub heap: Heap,
    obligations: BTreeMap<String, Vec<Obligation>>,
    pub max_depth: usize,
    pub fuel: u64,
}

impl<'m> Vm<'m> {
    pub fn new(module: &'m Module) -> Self {
        Self::with_heap(module, Heap::default())
    }

    pub fn with_heap(module: &'m Module, heap: Heap) -> Self {
        let obligations = module
            .functions
            .iter()
            .map(|f| (f.name.clone(), analyze(f).obligations))
            .collect();
        Self {
            module,
            heap,
            obligations,
            max_depth: 64,
            fuel: 1_000_000,
        }
    }

    pub fn into_heap(self) -> Heap {
        self.heap
    }

    pub fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, VmError> {
        self.call_depth(name, args, 0)
    }

    fn check_args(&self, f: &Function, args: &[Value]) -> Result<(), VmError> {
        if args.len() != f.params.len() {
            return Err(VmError::ArityMismatch {
                func: f.name.clone(),
                expected: f.params.len(),
                found: args.len(),
            });
        }
        for (i, (p, a)) in f.params.iter().zip(args).enumerate() {
            let ok = match (p, a) {
                (ParamType::I64, Value::I64(_)) | (ParamType::Bool, Value::Bool(_)) => true,
                (ParamType::Object(t), Value::Obj(id)) => self
                    .heap
                    .get(*id)
                    .map(|o| o.type_tag == *t)
                    .unwrap_or(false),
                _ => false,
            };
            if !ok {
                return Err(VmError::ParamTypeMismatch {
                    func: f.name.clone(),
                    index: i,
                    expected: *p,
                });
            }
        }
        for ob in self
            .obligations
            .get(&f.name)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
        {
            match ob {
                Obligation::DistinctMutableParams { a, b } => {
                    if let (Some(Value::Obj(x)), Some(Value::Obj(y))) =
                        (args.get(*a as usize), args.get(*b as usize))
                    {
                        if x == y {
                            return Err(VmError::AliasedUniqueParams {
                                func: f.name.clone(),
                                a: *a,
                                b: *b,
                                object: *x,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn call_depth(&mut self, name: &str, args: Vec<Value>, depth: usize) -> Result<Value, VmError> {
        if depth > self.max_depth {
            return Err(VmError::CallDepthExceeded(depth));
        }
        let f = self
            .module
            .function(name)
            .ok_or_else(|| VmError::FunctionNotFound(name.to_string()))?
            .clone();
        self.check_args(&f, &args)?;
        let mut regs = vec![Value::Consumed; f.regs as usize];
        for (i, a) in args.into_iter().enumerate() {
            *regs.get_mut(i).ok_or(VmError::InvalidReg(Reg(i as u16)))? = a;
        }
        self.exec(&f, &mut regs, depth)
    }

    fn get(regs: &[Value], r: Reg, op: usize) -> Result<&Value, VmError> {
        match regs.get(r.0 as usize) {
            None => Err(VmError::InvalidReg(r)),
            Some(Value::Consumed) => Err(VmError::UseAfterConsume { op, reg: r }),
            Some(v) => Ok(v),
        }
    }

    fn set(regs: &mut [Value], r: Reg, v: Value) -> Result<(), VmError> {
        *regs.get_mut(r.0 as usize).ok_or(VmError::InvalidReg(r))? = v;
        Ok(())
    }

    fn i64_of(regs: &[Value], r: Reg, op: usize) -> Result<i64, VmError> {
        match Self::get(regs, r, op)? {
            Value::I64(x) => Ok(*x),
            _ => Err(VmError::TypeMismatch { op }),
        }
    }

    fn obj_of(regs: &[Value], r: Reg, op: usize) -> Result<ObjectId, VmError> {
        match Self::get(regs, r, op)? {
            Value::Obj(x) => Ok(*x),
            _ => Err(VmError::TypeMismatch { op }),
        }
    }

    fn exec(&mut self, f: &Function, regs: &mut [Value], depth: usize) -> Result<Value, VmError> {
        for (i, op) in f.body.iter().enumerate() {
            if self.fuel == 0 {
                return Err(VmError::OutOfFuel);
            }
            self.fuel -= 1;
            match op {
                Op::ConstI64 { value, dest } => Self::set(regs, *dest, Value::I64(*value))?,
                Op::ConstBool { value, dest } => Self::set(regs, *dest, Value::Bool(*value))?,
                Op::Move { src, dest } => {
                    let v = Self::get(regs, *src, i)?.clone();
                    Self::set(regs, *src, Value::Consumed)?;
                    Self::set(regs, *dest, v)?;
                }
                Op::Consume { reg } => {
                    Self::get(regs, *reg, i)?;
                    Self::set(regs, *reg, Value::Consumed)?;
                }
                Op::AddChecked { a, b, dest } => {
                    let r = Self::i64_of(regs, *a, i)?
                        .checked_add(Self::i64_of(regs, *b, i)?)
                        .ok_or(VmError::Overflow { op: i })?;
                    Self::set(regs, *dest, Value::I64(r))?;
                }
                Op::SubChecked { a, b, dest } => {
                    let r = Self::i64_of(regs, *a, i)?
                        .checked_sub(Self::i64_of(regs, *b, i)?)
                        .ok_or(VmError::Overflow { op: i })?;
                    Self::set(regs, *dest, Value::I64(r))?;
                }
                Op::Ge { a, b, dest } => {
                    let r = Self::i64_of(regs, *a, i)? >= Self::i64_of(regs, *b, i)?;
                    Self::set(regs, *dest, Value::Bool(r))?;
                }
                Op::LoadField { obj, field, dest } => {
                    let id = Self::obj_of(regs, *obj, i)?;
                    let v = self.heap.field(id, *field)?.clone();
                    Self::set(regs, *dest, v)?;
                }
                Op::StoreField { obj, field, value } => {
                    let id = Self::obj_of(regs, *obj, i)?;
                    let v = Self::get(regs, *value, i)?.clone();
                    self.heap.set_field(id, *field, v)?;
                }
                Op::Assert { cond, error_code } => match Self::get(regs, *cond, i)? {
                    Value::Bool(true) => {}
                    Value::Bool(false) => return Err(VmError::AssertFailed { code: *error_code }),
                    _ => return Err(VmError::TypeMismatch { op: i }),
                },
                Op::Call { func, args, dest } => {
                    let mut vals = Vec::with_capacity(args.len());
                    for a in args {
                        vals.push(Self::get(regs, *a, i)?.clone());
                    }
                    let r = self.call_depth(func, vals, depth + 1)?;
                    if let Some(d) = dest {
                        Self::set(regs, *d, r)?;
                    }
                }
                Op::Return { value } => {
                    return match value {
                        Some(r) => Ok(Self::get(regs, *r, i)?.clone()),
                        None => Ok(Value::Unit),
                    };
                }
            }
        }
        Ok(Value::Unit)
    }
}

#[cfg(test)]
mod tests {
    use super::super::ast::examples::*;
    use super::*;

    fn wallets(vm: &mut Vm, a: i64, b: i64) -> (ObjectId, ObjectId) {
        let x = vm.heap.alloc(WALLET);
        let y = vm.heap.alloc(WALLET);
        vm.heap.set_field(x, BALANCE, Value::I64(a)).unwrap();
        vm.heap.set_field(y, BALANCE, Value::I64(b)).unwrap();
        (x, y)
    }

    #[test]
    fn transfer_moves_money_and_guards() {
        let m = payments_module();
        let mut vm = Vm::new(&m);
        let (a, b) = wallets(&mut vm, 1000, 200);
        assert_eq!(
            vm.call(
                "Transfer",
                vec![Value::Obj(a), Value::Obj(b), Value::I64(300)]
            ),
            Ok(Value::Bool(true))
        );
        assert_eq!(
            (vm.heap.field_i64(a, BALANCE), vm.heap.field_i64(b, BALANCE)),
            (Some(700), Some(500))
        );
        assert_eq!(
            vm.call(
                "Transfer",
                vec![Value::Obj(a), Value::Obj(b), Value::I64(99_999)]
            ),
            Err(VmError::AssertFailed {
                code: ERR_INSUFFICIENT
            })
        );
    }

    #[test]
    fn self_transfer_is_rejected_as_aliasing_instead_of_minting_money() {
        let m = payments_module();
        let mut vm = Vm::new(&m);
        let (a, _) = wallets(&mut vm, 1000, 0);
        let r = vm.call(
            "Transfer",
            vec![Value::Obj(a), Value::Obj(a), Value::I64(300)],
        );
        assert_eq!(
            r,
            Err(VmError::AliasedUniqueParams {
                func: "Transfer".into(),
                a: 0,
                b: 1,
                object: a
            })
        );
        assert_eq!(vm.heap.field_i64(a, BALANCE), Some(1000));
    }

    #[test]
    fn without_the_check_the_documented_order_mints_money() {
        // Reproduce the hazard the obligation protects against: run the same
        // body through a VM whose obligations are stripped.
        let m = payments_module();
        let mut vm = Vm::new(&m);
        vm.obligations.clear();
        let (a, _) = wallets(&mut vm, 1000, 0);
        vm.call(
            "Transfer",
            vec![Value::Obj(a), Value::Obj(a), Value::I64(300)],
        )
        .unwrap();
        assert_eq!(vm.heap.field_i64(a, BALANCE), Some(1300));
    }

    #[test]
    fn calls_and_type_checks() {
        let m = payments_module_v2();
        let mut vm = Vm::new(&m);
        let (a, b) = wallets(&mut vm, 500, 0);
        assert_eq!(
            vm.call(
                "Transfer",
                vec![Value::Obj(a), Value::Obj(b), Value::I64(500)]
            ),
            Ok(Value::Bool(true))
        );
        assert_eq!(vm.heap.field_i64(b, BALANCE), Some(500));
        assert_eq!(
            vm.call(
                "Transfer",
                vec![Value::Obj(a), Value::I64(1), Value::I64(1)]
            ),
            Err(VmError::ParamTypeMismatch {
                func: "Transfer".into(),
                index: 1,
                expected: ParamType::Object(WALLET)
            })
        );
        assert_eq!(
            vm.call("Nope", vec![]),
            Err(VmError::FunctionNotFound("Nope".into()))
        );
        assert!(matches!(
            vm.call("Transfer", vec![]),
            Err(VmError::ArityMismatch { .. })
        ));
    }

    #[test]
    fn use_after_consume_is_caught_at_runtime_too() {
        let m = Module {
            name: "M".into(),
            functions: vec![transfer_use_after_consume()],
        };
        let mut vm = Vm::new(&m);
        let (a, b) = wallets(&mut vm, 1000, 0);
        assert!(matches!(
            vm.call(
                "Transfer",
                vec![Value::Obj(a), Value::Obj(b), Value::I64(1)]
            ),
            Err(VmError::UseAfterConsume { reg: Reg(2), .. })
        ));
    }
}
