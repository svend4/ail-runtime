# Example: Transfer & Refund

## High-level (.ail style)

```ail
node Transfer(from: Wallet, to: Wallet, amount: i64) -> bool {
    let from_bal = from.balance;
    let to_bal   = to.balance;

    assert from_bal >= amount;

    from.balance = from_bal - amount;
    to.balance   = to_bal + amount;

    return true;
}

node Refund(from: Wallet, to: Wallet, amount: i64) -> bool {
    let from_bal = from.balance;
    let to_bal   = to.balance;

    assert from_bal >= amount;

    from.balance = from_bal - amount;
    to.balance   = to_bal + amount;

    return true;
}
```

## Conflict

```text
Transfer  ✕  Refund
```

Cannot run both at the same time on overlapping wallets.

## Binary AST (Transfer)

```text
LoadField  from.balance   → Reg(3)
LoadField  to.balance     → Reg(4)
Ge         Reg(3) >= amount → Reg(5)
Assert     Reg(5)
SubChecked Reg(3) - amount  → Reg(6)
StoreField from.balance   ← Reg(6)
AddChecked Reg(4) + amount  → Reg(7)
StoreField to.balance     ← Reg(7)
ConstBool  true           → Reg(8)
Return     Reg(8)
```
