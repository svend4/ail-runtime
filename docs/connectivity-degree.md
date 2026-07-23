# Connectivity Degree (1D / 2D / 3D)

Additional parameter for every edge that describes how open, fluid or rigid the connection is.

## Definition

```rust
enum ConnectivityDegree {
    D1,  // sparsity, weak links
    D2,  // fluidity / jamming zone
    D3,  // rigidity, saturation
}
```

An edge now carries three axes of description:

1. **Star Gate symbol** — character of the relation
2. **EdgeKind** — role in the system (Control, Data, Ownership…)
3. **ConnectivityDegree** — how open / fluid / rigid the link is

## Quantitative measure: saturation

Instead of assigning degree only by symbol, we can compute a continuous saturation value in `[0, 1]`.

### Recommended formula

For an edge `e = (u, v)`:

```text
s(e) = 2 * |common_neighbors(u, v)| / (deg(u) + deg(v))
```

Weighted variant:

```text
s(e) = sum(weight(w) for w in common) / sqrt(deg(u) * deg(v))
```

### Thresholds

| saturation | degree | Meaning                        |
|------------|--------|--------------------------------|
| 0.00–0.25  | D1     | Weak / open connection         |
| 0.25–0.75  | D2     | Jamming / fluid working zone   |
| 0.75–1.00  | D3     | Rigid / saturated connection   |

D2 is the main operational zone of the system.

## Mapping from Star Gate symbols

| Symbol     | Recommended degree | Reason                              |
|------------|--------------------|-------------------------------------|
| `·` `—`    | D1                 | Minimal connection                  |
| `→`        | D1 or D2           | Depends on strength of the flow     |
| `∠` `△`   | D2                 | Local structures, influence         |
| `◇` `□̸`  | D2                 | Stable but still flexible           |
| `▲`        | D2–D3              | Unique impact (start of jamming)    |
| `✕`        | D3                 | Hard conflict                       |
| `⬡`        | D3                 | Atomic group                        |
| Dense figures | D3              | Maximum saturation                  |

When both symbol and computed saturation are available, prefer the computed value and use the symbol as a semantic label.

## Conceptual scale

```text
1D ──────────────────────── 2D ──────────────────────── 3D
Sparsity                   Fluidity / Jamming          Rigidity
Independent sets           Transitional regimes        Cliques
```

- **D1** — almost no constraints, easy to break
- **D2** — main working zone (ownership tension, evolution, information flow)
- **D3** — strong guarantees, atomicity, explicit conflicts

## Marginality ratio (inspired by jamming theory)

A simple diagnostic for a subgraph:

```text
marginality = N / D
```

where:
- `N` ≈ number of shared / triangular structures
- `D` ≈ number of edge variations or degree of freedom

Interpretation:
- close to 0.5 → subgraph is near the critical (D2) regime
- ≪ 0.5 → too sparse (D1-like)
- ≫ 0.5 → over-saturated (D3-like)

This can be used as a health check during hot-swap or graph validation.

## Usage

### Ownership Analyzer
- D1 edges impose almost no restrictions
- D2 edges are the main zone of borrow/own checks
- D3 edges require the strictest guarantees

### Hot-Swap
- Changing D1 links is mostly safe
- Changing D2 links requires weakening checks
- Changing D3 links has the strongest constraints

### VizGraph
- D1 — thin, dim lines
- D2 — normal lines of medium brightness
- D3 — thick, bright lines, possibly with special outline

### Generation / Diffusion
- First build a D1 skeleton
- Then saturate with D2 relations
- Finally fix D3 groups

## Minimal Rust helpers

```rust
#[derive(Clone, Copy, Debug, PartialEq)]
enum ConnectivityDegree {
    D1,
    D2,
    D3,
}

fn saturation(u_deg: usize, v_deg: usize, common: usize) -> f64 {
    if u_deg + v_deg == 0 {
        return 0.0;
    }
    2.0 * common as f64 / (u_deg + v_deg) as f64
}

fn degree_from_saturation(s: f64) -> ConnectivityDegree {
    if s < 0.25 {
        ConnectivityDegree::D1
    } else if s <= 0.75 {
        ConnectivityDegree::D2
    } else {
        ConnectivityDegree::D3
    }
}
```

## Example

```text
Phone-X1.Price  →  Transfer        degree: D2   (s ≈ 0.4)
Transfer        ✕  Refund          degree: D3   (s ≈ 0.9)
Article         □̸  Phone-X1        degree: D2   (s ≈ 0.5)
Devices         ◇  Payments        degree: D2
payments_module ⬡  (group)         degree: D3
temporary link  —  (draft)         degree: D1   (s ≈ 0.1)
```

## Relation to Ramsey theory and Jamming

- **D1** corresponds to the sparse extreme (independent-set side of Ramsey)
- **D3** corresponds to the dense extreme (clique side of Ramsey)
- **D2** corresponds to the transitional / jamming regime studied in complex systems physics (Parisi–Zamponi 2026 and related work)

The continuous saturation value is a practical analogue of the interpolating function f(t) that appears in the jamming scaling theory.
