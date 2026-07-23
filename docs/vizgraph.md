# VizGraph — Visualization Layer

## Goal

Turn Binary AST + Ownership + Lineage into an interactive graph that a human can travel through.

## Core Structures

```rust
struct VizGraph {
    nodes: HashMap<NodeId, VizNode>,
    edges: Vec<VizEdge>,
    layers: AnnotationLayers,
    meta: GraphMeta,
}

enum NodeKind {
    Op { op_index: u32, op_kind: OpKind },
    Value { value_id: ValueId },
    HyperGroup { hyper_id: u32 },
    Function { func_id: u32 },
    Domain { code: String },
}

enum EdgeKind {
    Control,
    Data,
    Ownership { loan_kind: LoanKind },
    HyperMember,
    Diff { change: DiffChange },
}
```

## Annotation Layers

```rust
struct AnnotationLayers {
    traveler: Option<NodeId>,
    ownership: HashMap<NodeId, OwnershipState>,
    pheromone: HashMap<NodeId, f32>,
    hyper_groups: HashMap<u32, Vec<NodeId>>,
    errors: HashMap<NodeId, Vec<String>>,
    diff: HashMap<NodeId, DiffChange>,
    epoch: Option<u32>,
}
```

## Visual Rules (short)

| Element            | Color / Style              |
|--------------------|----------------------------|
| Owned              | Green                      |
| Borrowed           | Cyan                       |
| BorrowedMut        | Orange                     |
| Consumed / Conflict| Red                        |
| Traveler           | Bright glow + ring         |
| Hyper-group        | Transparent purple hull    |
| High pheromone     | Thicker / brighter edge    |

## Render Order

1. Hyper-groups (background)
2. Edges
3. Nodes
4. Traveler
5. Error markers
