# Step 1 — Relation Graph inside ShardActor

## Goal

Every `ShardActor` owns a `connectivity::Graph` that describes relations between its functions (and later values).

## What was added

```text
ShardActorWithGraph
  ├── module: BinaryModule          // executable semantics
  ├── history: Vec<BinaryModule>    // lineage
  ├── relations: Graph              // connectivity view
  └── fn_nodes: name → NodeId       // map functions to graph nodes
```

## API

- `new(module)` — builds actor and creates one node per function
- `link_functions(from, to, symbol, kind, weight)` — adds a typed edge and recomputes saturation
- `print_relations()` — human-readable dump of the connectivity view

## Why this matters

- Hot-swap can later consult the same graph that describes conflicts and data-flow
- Ownership analysis can see which functions interact
- VizGraph and diagnostics have a single source of truth inside the actor

## Next step

Step 2: make `hot_swap` automatically call `hot_swap_risk` using this graph.
