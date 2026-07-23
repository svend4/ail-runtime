# Hyper-Edges

## Idea

Ordinary edge connects two nodes.
Hyper-edge connects multiple participants at once and can carry its own body of operations.

## In Binary AST

```rust
enum Op {
    // ... ordinary ops

    Hyper {
        kind: HyperKind,
        participants: Vec<Reg>,
        body: Vec<Op>,
        meta: HyperMeta,
    },
}

enum HyperKind {
    AtomicUpdate,
    OwnershipGroup,
    MultiAssert,
    MultiCall { func_id: u32 },
    Custom(String),
}

struct HyperMeta {
    label: Option<String>,
    atomic: bool,
    invalidates_on_move: bool,
}
```

## Example (.ail style)

```ail
hyper AtomicTransfer(from, to, amount)
    invariant(total_balance_unchanged)
    atomic
    invalidates_on_move
{
    let from_bal = from.balance;
    let to_bal   = to.balance;

    assert from_bal >= amount;

    from.balance = from_bal - amount;
    to.balance   = to_bal + amount;
}
```

## Benefits

- Explicit atomicity
- Group ownership rules
- Contracts that apply to the whole group
- Cleaner analysis for multi-object operations

## When to introduce

Only when real multi-object atomic scenarios appear.
For the current MVP ordinary `Op`s are enough.
