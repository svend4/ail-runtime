# Implementation Status

## What is fully implemented now

### Core Runtime (`mvp/src/main.rs`)
- Binary AST + interpreter
- Actor / ShardActor
- Ownership (basic move / consume)
- Hot-Swap with weakening check
- Lineage (generation, parent_hash, origin)
- Calls the connectivity integration demo

### Connectivity Layer (`mvp/src/connectivity.rs`)
- Graph structure with adjacency
- Automatic saturation computation
- D1 / D2 / D3 degree assignment
- Degree-aware borrow helpers
- Hot-Swap risk matrix (`hot_swap_risk`)
- Marginality ratio diagnostic
- Unit test for triangle saturation

### Integration Demo (`mvp/src/integration_demo.rs`) — NEW
- Builds a relation graph for PaymentsModule:
  - Transfer ✕ Refund
  - Transfer → CheckBalance
  - CheckBalance — Balance
- Prints saturation + degree for every edge
- Demonstrates connectivity-aware hot-swap decisions:
  - Downgrading a conflict edge → Critical / rejected
  - Changing a normal data edge → Low/Medium / allowed

### Documentation
- specification.md
- visual-edges.md
- architecture.md
- comparison.md
- edge-rules.md
- germes-mapping.md
- hot-swap.md
- ownership-analyzer.md
- vizgraph.md
- hyper-edges.md
- connectivity-degree.md
- saturation-usage.md
- IMPLEMENTATION_STATUS.md (this file)

## How the pieces fit together

```text
Binary AST functions
        ↔
Relation Graph (connectivity::Graph)
        → saturation() → D1/D2/D3
            → Ownership strictness
            → Hot-Swap risk
            → Marginality health check
            → (future) VizGraph styles
```

## Current demo flow (`cargo run`)

1. Classic Transfer execution + insufficient funds
2. Compatible hot-swap + lineage print
3. Weakening rejection (function removed)
4. Connectivity integration:
   - Relation graph report
   - Risk-based swap decisions

## Next natural steps

1. Store the relation graph inside `ShardActor` so every module owns its connectivity view
2. Make `ShardActor::hot_swap` call `hot_swap_risk` automatically
3. Weighted saturation (use edge weights / pheromone)
4. Expose `marginality_ratio` as a runtime metric after each swap
5. First VizGraph pass that colours edges by degree
