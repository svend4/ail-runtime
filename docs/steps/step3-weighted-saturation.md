# Step 3 — Weighted Saturation

## Goal

Make saturation depend not only on topology, but also on **edge weight** (importance / pheromone).

## Formula

Unweighted (old):

```text
s(u,v) = 2 * |N(u) ∩ N(v)| / (deg(u) + deg(v))
```

Weighted (new):

```text
common_weight = average of weights of edges into common neighbours
weighted_deg(x) = sum of weights of edges incident to x

s_w(u,v) = 2 * common_weight / (weighted_deg(u) + weighted_deg(v))
```

Clamped to `[0, 1]`.

## API additions

```rust
graph.reinforce_edge(from, to, delta);  // pheromone boost / decay
graph.saturation(u, v);                 // now weighted
```

`add_edge(..., weight)` stores the weight and uses it immediately.

## Why it matters

- Frequently used relations become more “saturated” (can migrate toward D3)
- Temporary / weak links stay in D1 even inside a dense topology
- Hot-swap risk and ownership strictness follow real usage, not only structure

## Next step

Step 4: expose `marginality_ratio` as a runtime metric after each swap.
