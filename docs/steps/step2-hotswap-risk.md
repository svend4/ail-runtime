# Step 2 — Hot-Swap consults connectivity risk

## Goal

`hot_swap` no longer looks only at function names.
It also inspects the relation graph and refuses (or warns on) dangerous changes.

## Policy (MVP)

When a candidate module is proposed:

1. **Weakening check** — no existing function may disappear.
2. **Connectivity risk**:
   - any affected **D3** edge → `Critical` → reject
   - any affected **D2** edge → `High` → warn (full re-check in production)
   - only **D1** affected → `Low` / `Medium`
3. On success: accept, push lineage, rebuild relation graph.

## Code

See `mvp/src/actor_with_graph.rs`:

- `estimate_swap_risk`
- `hot_swap` (now risk-aware)

## Demo behaviour

- Keeping all functions → accepted
- Removing a function that participates in a Conflict (D3) edge → rejected

## Next step

Step 3: weighted saturation (use edge weights / pheromone intensity).
