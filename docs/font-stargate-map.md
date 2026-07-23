# Font ↔ Star Gate Map (myFront4Solan4)

The custom font **myFront4Solan4** is a graphical carrier of the Star Gate edge alphabet.
Glyphs are 16×N bitmaps; the important symbolic range is codes 34–42.
Letters/digits in this font are additional geometric motifs (not standard Latin shapes).

## Core glyph map

| Code | Input char | Visual character              | Star Gate | Degree | Meaning                                      |
|------|------------|-------------------------------|-----------|--------|----------------------------------------------|
| 34   | `"`        | sparse horizontal stripes     | `·` / `—` | D1     | weak / draft / open link                     |
| 40   | `(`        | interrupted stripes           | `∠`      | D2     | angle / local turn                           |
| 39   | `'`        | diamond lattice               | `◇`      | D2     | stable bilateral link                        |
| 42   | `*`        | quarter tiles                 | `△` / tile| D2    | minimal assemble unit / small cycle          |
| 38   | `&`        | frame with windows            | `□̸`     | D2     | shared borrow / overlap                      |
| 36   | `$`        | mirrored top/bottom halves    | lever     | D2–D3  | clique ↔ independent-set balance (lever)     |
| 41   | `)`        | solid split blocks            | `✕`      | D3     | conflict / hard cut                          |
| 37   | `%`        | dense alternating bars        | dense     | D3     | high saturation                              |
| 35   | `#`        | full checker / grid           | `⬡`-like | D3     | hyper-group / maximum saturation             |

## Extended motifs (A–Z, a–z, 0–9)

These codes render abstract geometric patterns (frames, diagonals, partial fills).
Recommended use:

- **Node type labels** in UI (e.g. T = Transfer, R = Refund) when the font is loaded
- **Modifiers** on edges (optional future layer)
- Not required for core degree / ownership logic

## Four axes of an edge

```text
1. Star Gate symbol   — semantic character of the relation
2. EdgeKind           — Control / Data / Ownership / …
3. ConnectivityDegree — D1 / D2 / D3 (computed or declared)
4. glyph code         — font id for print / UI / M2M text
```

## Rendering rules (VizGraph)

| Degree | Preferred glyphs     | Line style (existing) |
|--------|----------------------|------------------------|
| D1     | `"`                  | dashed, thin, low alpha |
| D2     | `'` `*` `&` `(` `$`  | solid, medium           |
| D3     | `)` `%` `#`          | bold, thick, high alpha |

## Lever glyph

`$` is the visual for **LeverBalance** (C ↔ I):
- left/top half ≈ clique pressure
- right/bottom half ≈ independence pressure

Use in dashboards next to `marginality_ratio` and `LeverBalance.product`.

## Source files

- Font: `myFront4Solan4.ttf` (user asset)
- Glyph table dump: accompanying JSON/txt with 16-bit row arrays
- Logic: `docs/visual-edges.md`, `docs/connectivity-degree.md`, `docs/lever-balance.md`
