# Saturation & Connectivity Degree — Full Usage Spec

## 1. Purpose

Turn the qualitative Star Gate scale (D1 / D2 / D3) into a **computable** property of every edge and every local subgraph.

Inspired by the interpolating function f(t) ∈ [0,1] from jamming theory (Parisi–Zamponi 2026).

## 2. Core Quantities

### 2.1 Edge saturation

For edge e = (u, v):

```text
s(e) = 2 * |N(u) ∩ N(v)| / (deg(u) + deg(v))
```

Weighted version:

```text
s(e) = Σ weight(w) for w ∈ N(u) ∩ N(v)  /  sqrt(deg(u) * deg(v))
```

Range: [0.0, 1.0]

### 2.2 Degree assignment

| saturation | degree | Zone                |
|------------|--------|---------------------|
| [0.00, 0.25) | D1   | Sparse / open       |
| [0.25, 0.75] | D2   | Jamming / fluid     |
| (0.75, 1.00] | D3   | Rigid / saturated   |

### 2.3 Subgraph marginality ratio

```text
marginality(G') = N / D
```

- N ≈ number of shared-neighbor pairs (or triangles)
- D ≈ number of possible edges or degrees of freedom inside G'

Target critical value ≈ 0.5 (jamming point).

## 3. Data Model

```rust
enum ConnectivityDegree { D1, D2, D3 }

enum StarGateSymbol { Dot, Simple, Directed, Angle, Triangle, Diamond, Shared, Unique, Conflict, Hyper }

enum EdgeKind { Control, Data, Ownership, HyperMember, Diff }

struct Edge {
    from: NodeId,
    to: NodeId,
    symbol: StarGateSymbol,
    kind: EdgeKind,
    saturation: f64,
    degree: ConnectivityDegree,
    weight: f32,
}
```

## 4. Ownership rules by degree

| Degree | Shared borrow | Unique borrow | Move | Conflict tolerance |
|--------|---------------|---------------|------|--------------------|
| D1     | always        | usually ok    | ok   | high               |
| D2     | ok            | restricted    | check| medium             |
| D3     | limited       | almost never  | hard | low                |

## 5. Hot-Swap risk matrix

| old \ new | D1        | D2          | D3            |
|-----------|-----------|-------------|---------------|
| D1        | Low       | Low–Medium  | Medium        |
| D2        | Medium    | Medium      | High          |
| D3        | High      | Critical    | Critical      |

## 6. Algorithmic pipeline

1. Build or update graph
2. For every edge compute saturation → assign degree
3. Run ownership analysis (degree-aware)
4. On hot-swap candidate:
   - compute risk from degree transition
   - if risk ≥ High → full weakening + ownership re-check
   - if risk = Critical → reject or require explicit override
5. Optionally compute marginality of affected subgraph
6. Commit new generation + lineage

## 7. Visualisation mapping

| Degree | Line style              |
|--------|-------------------------|
| D1     | thin, dashed, low alpha |
| D2     | normal, solid           |
| D3     | thick, bright, outline  |

## 8. Design principles

- Saturation is **computed**, not declared
- Symbol remains semantic label
- Degree is the operational control knob
- D2 is the default working regime of the system
- Marginality ≈ 0.5 is a health indicator, not a hard constraint
