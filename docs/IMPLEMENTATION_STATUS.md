# Implementation Status

## What is fully implemented now

### Core Runtime (mvp/src/main.rs)
- Binary AST + interpreter
- Actor / ShardActor
- Ownership (basic move / consume)
- Hot-Swap with weakening check
- Lineage (generation, parent_hash, origin)

### Connectivity Layer (mvp/src/connectivity.rs) — NEW
- Graph structure with adjacency
- Automatic saturation computation
- D1 / D2 / D3 degree assignment
- Degree-aware borrow helpers
- Hot-Swap risk matrix
- Marginality ratio diagnostic
- Unit test for triangle saturation

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
- connectivity-degree.md (with formula + thresholds)
- saturation-usage.md (full operational spec)

## How the pieces fit together

```text
Graph / Edges
    → saturation() → ConnectivityDegree (D1/D2/D3)
        → Ownership checks (strictness depends on degree)
        → Hot-Swap risk matrix
        → VizGraph styling
        → Marginality health check
```

## Next natural steps

1. Integrate connectivity::Graph into the main Actor so that Binary AST edges also carry saturation
2. Make hot_swap() consult hot_swap_risk() before accepting a candidate
3. Expose marginality_ratio() as a runtime diagnostic
4. Add more sophisticated weighted saturation
5. Visualisation layer that reads degree for line styles
