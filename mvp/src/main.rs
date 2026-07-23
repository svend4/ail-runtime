use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use sha2::{Sha256, Digest};

// ============================================================
// 1. Базовые типы
// ============================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Reg(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct ObjectId(u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Value {
    I64(i64),
    Bool(bool),
    ObjectRef(ObjectId),
    Consumed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum Op {
    ConstI64 { value: i64, dest: Reg },
    ConstBool { value: bool, dest: Reg },
    Move { src: Reg, dest: Reg },
    Consume { reg: Reg },
    AddChecked { a: Reg, b: Reg, dest: Reg },
    SubChecked { a: Reg, b: Reg, dest: Reg },
    Ge { a: Reg, b: Reg, dest: Reg },
    LoadField { obj: Reg, field: u16, dest: Reg },
    StoreField { obj: Reg, field: u16, value: Reg },
    Assert { cond: Reg, error_code: u16 },
    Return { value: Option<Reg> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinaryFunction {
    name: String,
    params: u8,
    body: Vec<Op>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ModuleOrigin {
    Manual,
    Healed { reason: String, from_generation: u32 },
    Loaded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinaryModule {
    name: String,
    version: u32,
    functions: Vec<BinaryFunction>,
    content_hash: String,
    parent_hash: Option<String>,
    generation: u32,
    origin: ModuleOrigin,
}

// ============================================================
// 2. Состояние
// ============================================================

#[derive(Debug, Clone)]
struct Object {
    fields: HashMap<u16, Value>,
}

#[derive(Debug)]
struct ActorState {
    registers: Vec<Value>,
    objects: HashMap<ObjectId, Object>,
    next_object_id: u64,
}

impl ActorState {
    fn new(reg_count: usize) -> Self {
        Self {
            registers: vec![Value::Consumed; reg_count],
            objects: HashMap::new(),
            next_object_id: 1,
        }
    }

    fn alloc_object(&mut self) -> ObjectId {
        let id = ObjectId(self.next_object_id);
        self.next_object_id += 1;
        self.objects.insert(id, Object { fields: HashMap::new() });
        id
    }

    fn get_reg(&self, reg: Reg) -> Result<&Value, ExecError> {
        self.registers.get(reg.0 as usize).ok_or(ExecError::InvalidReg)
    }

    fn set_reg(&mut self, reg: Reg, value: Value) -> Result<(), ExecError> {
        let slot = self.registers.get_mut(reg.0 as usize).ok_or(ExecError::InvalidReg)?;
        *slot = value;
        Ok(())
    }
}

// ============================================================
// 3. Ошибки
// ============================================================

#[derive(Debug, PartialEq)]
enum ExecError {
    InvalidReg,
    TypeMismatch,
    Overflow,
    AssertFailed { code: u16 },
    UseAfterConsume,
    ObjectNotFound,
    FieldNotFound,
    FunctionNotFound,
}

// ============================================================
// 4. Интерпретатор
// ============================================================

fn expect_i64(v: &Value) -> Result<i64, ExecError> {
    match v {
        Value::I64(x) => Ok(*x),
        Value::Consumed => Err(ExecError::UseAfterConsume),
        _ => Err(ExecError::TypeMismatch),
    }
}

fn expect_bool(v: &Value) -> Result<bool, ExecError> {
    match v {
        Value::Bool(x) => Ok(*x),
        Value::Consumed => Err(ExecError::UseAfterConsume),
        _ => Err(ExecError::TypeMismatch),
    }
}

fn expect_obj(v: &Value) -> Result<ObjectId, ExecError> {
    match v {
        Value::ObjectRef(id) => Ok(*id),
        Value::Consumed => Err(ExecError::UseAfterConsume),
        _ => Err(ExecError::TypeMismatch),
    }
}

fn execute(func: &BinaryFunction, state: &mut ActorState) -> Result<Value, ExecError> {
    for op in &func.body {
        match op {
            Op::ConstI64 { value, dest } => {
                state.set_reg(*dest, Value::I64(*value))?;
            }
            Op::ConstBool { value, dest } => {
                state.set_reg(*dest, Value::Bool(*value))?;
            }
            Op::Move { src, dest } => {
                let val = match state.get_reg(*src)? {
                    Value::Consumed => return Err(ExecError::UseAfterConsume),
                    other => other.clone(),
                };
                state.set_reg(*src, Value::Consumed)?;
                state.set_reg(*dest, val)?;
            }
            Op::Consume { reg } => {
                state.set_reg(*reg, Value::Consumed)?;
            }
            Op::AddChecked { a, b, dest } => {
                let va = expect_i64(state.get_reg(*a)?)?;
                let vb = expect_i64(state.get_reg(*b)?)?;
                let result = va.checked_add(vb).ok_or(ExecError::Overflow)?;
                state.set_reg(*dest, Value::I64(result))?;
            }
            Op::SubChecked { a, b, dest } => {
                let va = expect_i64(state.get_reg(*a)?)?;
                let vb = expect_i64(state.get_reg(*b)?)?;
                let result = va.checked_sub(vb).ok_or(ExecError::Overflow)?;
                state.set_reg(*dest, Value::I64(result))?;
            }
            Op::Ge { a, b, dest } => {
                let va = expect_i64(state.get_reg(*a)?)?;
                let vb = expect_i64(state.get_reg(*b)?)?;
                state.set_reg(*dest, Value::Bool(va >= vb))?;
            }
            Op::LoadField { obj, field, dest } => {
                let obj_id = expect_obj(state.get_reg(*obj)?)?;
                let object = state.objects.get(&obj_id).ok_or(ExecError::ObjectNotFound)?;
                let value = object.fields.get(field).cloned().ok_or(ExecError::FieldNotFound)?;
                state.set_reg(*dest, value)?;
            }
            Op::StoreField { obj, field, value } => {
                let obj_id = expect_obj(state.get_reg(*obj)?)?;
                let val = match state.get_reg(*value)? {
                    Value::Consumed => return Err(ExecError::UseAfterConsume),
                    other => other.clone(),
                };
                let object = state.objects.get_mut(&obj_id).ok_or(ExecError::ObjectNotFound)?;
                object.fields.insert(*field, val);
            }
            Op::Assert { cond, error_code } => {
                let v = expect_bool(state.get_reg(*cond)?)?;
                if !v {
                    return Err(ExecError::AssertFailed { code: *error_code });
                }
            }
            Op::Return { value } => {
                return match value {
                    Some(reg) => Ok(state.get_reg(*reg)?.clone()),
                    None => Ok(Value::Bool(true)),
                };
            }
        }
    }
    Ok(Value::Bool(true))
}

// ============================================================
// 5. Hash + Lineage
// ============================================================

fn compute_content_hash(module: &BinaryModule) -> String {
    let mut hasher = Sha256::new();
    let data = serde_json::json!({
        "name": module.name,
        "functions": module.functions,
    });
    hasher.update(data.to_string().as_bytes());
    format!("{:x}", hasher.finalize())
}

// ============================================================
// 6. Actor + Hot-Swap
// ============================================================

struct ShardActor {
    state: ActorState,
    module: BinaryModule,
    history: Vec<BinaryModule>,
}

impl ShardActor {
    fn new(module: BinaryModule, reg_count: usize) -> Self {
        Self {
            state: ActorState::new(reg_count),
            module,
            history: Vec::new(),
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>) -> Result<Value, ExecError> {
        let func = self.module.functions.iter()
            .find(|f| f.name == name)
            .ok_or(ExecError::FunctionNotFound)?
            .clone();

        for (i, arg) in args.into_iter().enumerate() {
            self.state.set_reg(Reg(i as u16), arg)?;
        }

        execute(&func, &mut self.state)
    }

    fn hot_swap(&mut self, new_functions: Vec<BinaryFunction>, reason: &str) -> Result<(), String> {
        let mut candidate = BinaryModule {
            name: self.module.name.clone(),
            version: self.module.version + 1,
            functions: new_functions,
            content_hash: String::new(),
            parent_hash: Some(self.module.content_hash.clone()),
            generation: self.module.generation + 1,
            origin: ModuleOrigin::Healed {
                reason: reason.to_string(),
                from_generation: self.module.generation,
            },
        };
        candidate.content_hash = compute_content_hash(&candidate);

        // Weakening check
        for old_fn in &self.module.functions {
            if !candidate.functions.iter().any(|f| f.name == old_fn.name) {
                return Err(format!(
                    "Function `{}` was removed — weakening detected (gen {} → gen {})",
                    old_fn.name, self.module.generation, candidate.generation
                ));
            }
        }

        self.history.push(self.module.clone());
        self.module = candidate;

        println!(
            "Hot-swap accepted: gen {} → gen {}",
            self.module.generation - 1,
            self.module.generation
        );
        Ok(())
    }

    fn print_lineage(&self) {
        println!("=== Full Module Lineage ===");
        println!("Module: {}", self.module.name);
        println!();

        for prev in &self.history {
            print_generation(prev);
            println!("        |");
            println!("        v");
        }
        print_generation(&self.module);
        println!("===========================");
    }
}

fn print_generation(module: &BinaryModule) {
    let origin_str = match &module.origin {
        ModuleOrigin::Manual => "Manual".to_string(),
        ModuleOrigin::Loaded => "Loaded".to_string(),
        ModuleOrigin::Healed { reason, from_generation } => {
            format!("Healed from gen {} ({})", from_generation, reason)
        }
    };

    let short_hash = if module.content_hash.len() >= 12 {
        &module.content_hash[..12]
    } else {
        &module.content_hash
    };

    println!(
        "gen {:>2} | v{:<3} | {} | {}",
        module.generation,
        module.version,
        short_hash,
        origin_str
    );
}

// ============================================================
// 7. Demo
// ============================================================

const BALANCE: u16 = 0;

fn make_transfer_function() -> BinaryFunction {
    BinaryFunction {
        name: "Transfer".into(),
        params: 3,
        body: vec![
            Op::LoadField { obj: Reg(0), field: BALANCE, dest: Reg(3) },
            Op::Ge { a: Reg(3), b: Reg(2), dest: Reg(4) },
            Op::Assert { cond: Reg(4), error_code: 401 },
            Op::SubChecked { a: Reg(3), b: Reg(2), dest: Reg(5) },
            Op::StoreField { obj: Reg(0), field: BALANCE, value: Reg(5) },
            Op::LoadField { obj: Reg(1), field: BALANCE, dest: Reg(6) },
            Op::AddChecked { a: Reg(6), b: Reg(2), dest: Reg(7) },
            Op::StoreField { obj: Reg(1), field: BALANCE, value: Reg(7) },
            Op::ConstBool { value: true, dest: Reg(8) },
            Op::Return { value: Some(Reg(8)) },
        ],
    }
}

fn make_module(version: u32, functions: Vec<BinaryFunction>) -> BinaryModule {
    let mut module = BinaryModule {
        name: "PaymentsModule".into(),
        version,
        functions,
        content_hash: String::new(),
        parent_hash: None,
        generation: 0,
        origin: ModuleOrigin::Manual,
    };
    module.content_hash = compute_content_hash(&module);
    module
}

fn main() {
    println!("=== AIL Runtime MVP Demo ===\n");

    let module = make_module(1, vec![make_transfer_function()]);
    let mut actor = ShardActor::new(module, 16);

    // Create wallets
    let from_id = actor.state.alloc_object();
    let to_id = actor.state.alloc_object();
    actor.state.objects.get_mut(&from_id).unwrap().fields.insert(BALANCE, Value::I64(1000));
    actor.state.objects.get_mut(&to_id).unwrap().fields.insert(BALANCE, Value::I64(200));

    // Test 1: successful transfer
    println!("1. Successful transfer 300:");
    let result = actor.call("Transfer", vec![
        Value::ObjectRef(from_id),
        Value::ObjectRef(to_id),
        Value::I64(300),
    ]);
    println!("   Result: {:?}", result);
    println!("   From: {:?}", actor.state.objects[&from_id].fields.get(&BALANCE));
    println!("   To:   {:?}", actor.state.objects[&to_id].fields.get(&BALANCE));

    // Test 2: insufficient funds
    println!("\n2. Insufficient funds:");
    let result = actor.call("Transfer", vec![
        Value::ObjectRef(from_id),
        Value::ObjectRef(to_id),
        Value::I64(99999),
    ]);
    println!("   Result: {:?}", result);

    // Test 3: Hot-swap
    println!("\n3. Hot-swap (compatible):");
    let new_funcs = vec![make_transfer_function()];
    let _ = actor.hot_swap(new_funcs, "compatible update");

    // Test 4: Lineage
    println!();
    actor.print_lineage();

    // Test 5: Weakening rejected
    println!("\n5. Hot-swap (function removed — should fail):");
    let result = actor.hot_swap(vec![], "remove Transfer");
    println!("   Result: {:?}", result);

    println!("\n=== All demos finished ===");
}
