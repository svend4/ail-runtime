# Step 4 — Marginality as Runtime Metric

## Goal

After every structural change (especially hot-swap) the actor records and prints a health snapshot.

## RuntimeMetrics

```rust
struct RuntimeMetrics {
    generation: u32,
    marginality: f64,       // N/D style ratio
    edge_count: usize,
    d1_count: usize,
    d2_count: usize,
    d3_count: usize,
    risk_of_last_swap: Option<SwapRisk>,
}
```

## Interpretation of marginality

| marginality | Zone                        |
|-------------|-----------------------------|
| < 0.35      | sparse (D1-leaning)         |
| 0.35–0.65   | near-critical / jamming D2  |
| > 0.65      | over-saturated (D3-leaning) |

## When metrics are recorded

- At actor construction
- After each accepted `hot_swap`
- Optionally after manual `link_functions` batches

## API

```rust
actor.latest_metrics() -> Option<&RuntimeMetrics>
actor.metrics_log      // full history
```

## Why it matters

- Gives an immediate signal whether the module is drifting toward rigidity or sparsity
- Complements lineage: not only *what* changed, but *how healthy* the connectivity is
- Prepares the ground for VizGraph colouring and automated alerts

## Next step

Step 5: first VizGraph pass that colours edges by degree (D1/D2/D3).
