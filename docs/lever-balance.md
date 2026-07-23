# Lever Balance — Clique vs Independent Set

## Idea

Ramsey theory says that as a graph grows, either a large clique or a large independent set becomes inevitable.

In AIL Runtime we treat these two extremes as **inversely related pressures** on the same system — a lever / balance scale:

```text
more clique-pressure (D3)  ⇒  less room for independence (D1)
more independence (D1)     ⇒  less room for cliques (D3)
```

The operational goal is to stay in the middle (**D2 / jamming zone**), where neither side dominates.

## Proxies

| Symbol | Meaning                         | Computed from                |
|--------|---------------------------------|------------------------------|
| C      | clique pressure                 | share of D3 edges            |
| I      | independence pressure           | share of D1 edges            |
| D2     | working / fluid zone            | share of D2 edges            |

After normalising so that C + I = 1 (D2 is the residual working mass):

```text
lever_product = C * I
```

- Balanced lever: C ≈ I ≈ 0.5 ⇒ product ≈ 0.25
- Soft health window: |C − I| < 0.25 and product > 0.15

## Relation to previous work

- **Ramsey (Bradač / off-diagonal r(s,k))** — extremes become inevitable; we measure how close we are.
- **Jamming (Parisi–Zamponi)** — the useful regime is the critical middle; that is our D2 target.
- **Star Gate alphabet** — D1 symbols (open) vs D3 symbols (closed); D2 primitives live between them.

## Runtime use

After every structural change (link, reinforce, hot-swap):

1. Recompute edge degrees
2. Build `LeverBalance` from D1/D2/D3 counts
3. Log it next to `marginality_ratio`
4. Warn if the lever is tilted too far

Hard blocking on lever imbalance is optional (not enabled in MVP).

## E-commerce reading

- High I (many D1) → catalogue feels like a dump; reinforce similarity / bundle edges
- High C (many D3) → catalogue feels monolithic; split categories or weaken hard links
- Balanced → navigable graph of products, attributes and relations
