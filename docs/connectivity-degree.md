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

## Conceptual scale

```text
1D ──────────────────────── 2D ──────────────────────── 3D
Sparsity                   Fluidity / Jamming          Rigidity
Independent sets           Transitional regimes        Cliques
```

- **D1** — almost no constraints, easy to break
- **D2** — main working zone (ownership tension, evolution, information flow)
- **D3** — strong guarantees, atomicity, explicit conflicts

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

## Example

```text
Phone-X1.Price  →  Transfer        degree: D2
Transfer        ✕  Refund          degree: D3
Article         □̸  Phone-X1        degree: D2
Devices         ◇  Payments        degree: D2
payments_module ⬡  (group)         degree: D3
temporary link  —  (draft)         degree: D1
```

## Relation to Ramsey theory and Jamming

- **D1** corresponds to the sparse extreme (independent-set side of Ramsey)
- **D3** corresponds to the dense extreme (clique side of Ramsey)
- **D2** corresponds to the transitional / jamming regime studied in complex systems physics
