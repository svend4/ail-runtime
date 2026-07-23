# Ownership Analyzer

## Goal

Statically prove (or reject) that ownership rules are respected inside a function / module:

- Exactly one owner at any moment
- No use-after-consume
- No simultaneous `&` and `&mut`
- No mutation through shared borrow

## Core Concepts

```rust
enum OwnershipState {
    Owned,
    Borrowed { from: ValueId },
    BorrowedMut { from: ValueId },
    Consumed,
    Uninitialized,
}

enum LoanKind {
    Shared,    // &
    Unique,    // &mut
}
```

## Main Structure

```rust
struct OwnershipAnalyzer {
    states: HashMap<ValueId, OwnershipState>,
    active_loans: Vec<Loan>,
    errors: Vec<OwnershipError>,
    reg_to_value: HashMap<Reg, ValueId>,
}
```

## Key Checks

- `check_can_move`
- `check_can_borrow_shared`
- `check_can_borrow_mut`
- `check_can_mutate`
- `check_not_consumed`

## Typical Errors

| Error                      | Meaning                              |
|----------------------------|--------------------------------------|
| UseAfterConsume            | Value used after it was consumed     |
| MultipleMutBorrows         | Two `&mut` at the same time          |
| MoveWhileBorrowed          | Moving a value that is still borrowed|
| MutateThroughSharedBorrow  | Writing through `&`                  |
| MoveWhileInHyperEdge       | Move inside atomic group with flag   |

## Integration with Hot-Swap

Stage 2 of hot-swap runs the ownership analyzer on every function of the candidate module.
If any ownership error is found, the swap is rejected.

## Future Improvements

- Proper scopes / ending of loans
- Reborrowing rules
- Inter-procedural analysis
- Integration with contracts (`must_own`, `must_not_borrow`)
