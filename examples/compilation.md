# Compilation: .ail → Binary AST

## Supported Constructs (MVP)

```ail
node Name(param: Type, ...) -> ReturnType {
    let x = expr;
    assert condition;
    obj.field = expr;
    return expr;
}
```

## Compilation Rules

| .ail construct              | Binary AST                              |
|----------------------------|-----------------------------------------|
| Parameters                 | Reg(0), Reg(1), Reg(2)…                 |
| `let x = expr;`            | Evaluate expr → new register            |
| `obj.field`                | `Op::LoadField`                         |
| `obj.field = expr;`        | Evaluate expr → `Op::StoreField`        |
| `a + b` / `a - b`          | `AddChecked` / `SubChecked`             |
| `a >= b`                   | `Op::Ge`                                |
| `assert cond;`             | Evaluate cond → `Op::Assert`            |
| `return expr;`             | Evaluate expr → `Op::Return`            |

## Example: Transfer

**Input**
```ail
node Transfer(from: Wallet, to: Wallet, amount: i64) -> bool {
    let from_bal = from.balance;
    let to_bal   = to.balance;
    assert from_bal >= amount;
    from.balance = from_bal - amount;
    to.balance   = to_bal + amount;
    return true;
}
```

**Output (simplified)**
```text
Reg(0) = from
Reg(1) = to
Reg(2) = amount

LoadField  from.balance → Reg(3)
LoadField  to.balance   → Reg(4)
Ge         Reg(3) >= Reg(2) → Reg(5)
Assert     Reg(5)
SubChecked Reg(3) - Reg(2) → Reg(6)
StoreField from.balance ← Reg(6)
AddChecked Reg(4) + Reg(2) → Reg(7)
StoreField to.balance   ← Reg(7)
ConstBool  true → Reg(8)
Return     Reg(8)
```
