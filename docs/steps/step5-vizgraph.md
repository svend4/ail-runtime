# Step 5 — VizGraph (degree → visual style)

## Goal

Map every edge's `ConnectivityDegree` to a concrete visual style so a renderer (terminal, canvas, web) can draw the relation graph consistently.

## Style rules

| Degree | Colour              | Width        | Style   | Meaning                |
|--------|---------------------|--------------|---------|------------------------|
| D1     | cool grey-blue, low alpha | thin   | Dashed  | weak / open            |
| D2     | amber→teal by saturation | normal | Solid | jamming / working zone |
| D3     | red-orange, high alpha   | thick  | Bold   | rigid / conflict       |

Width also scales mildly with `weight` (pheromone).

## Data produced

```rust
VizScene {
    nodes: Vec<VizNode>,   // id, label, x, y, radius
    edges: Vec<VizEdge>,   // from, to, degree, saturation, weight, color, width, style
    title: String,
}
```

Layout for now is a simple circle. Real force-directed / hierarchical layout can replace it later.

## API

```rust
let scene = VizGraph::from_graph(&actor.relations, &labels, "PaymentsModule");
VizGraph::print_scene(&scene);
```

## Integration point

After `hot_swap` or `link_functions`, build a fresh `VizScene` from the actor's relation graph and either:

- dump to terminal (current MVP),
- serialize to JSON for a front-end,
- or feed a GPU/canvas renderer.

## Status of the 5-step plan

1. ✅ Actor owns relation Graph
2. ✅ Hot-swap consults connectivity risk
3. ✅ Weighted saturation
4. ✅ Marginality runtime metrics
5. ✅ VizGraph degree styling
