# Step 6 — Lever Balance

## Goal

Treat clique-side (D3) and independent-set-side (D1) pressures as inversely related and expose a simple diagnostic.

## Added

- `mvp/src/lever.rs` — `LeverBalance`
- `docs/lever-balance.md` — concept + Ramsey/jamming link
- `examples/ecommerce_lever.md` — product-graph reading

## Formula

```text
C = share of D3 (normalised)
I = share of D1 (normalised)
product = C * I
tilted  = |C - I| >= 0.25  OR  product <= 0.15
```

## Status of steps

1. Actor owns graph ✅
2. Risk-aware hot-swap ✅
3. Weighted saturation ✅
4. Runtime metrics ✅
5. VizGraph styles ✅
6. Lever balance ✅
