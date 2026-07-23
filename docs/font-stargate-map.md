# Font ↔ Star Gate Map (myFront4Solan4 / Solan3)

Custom fonts **myFront4Solan4** and **myFront4Solan3** map ordinary keyboard characters to geometric glyphs. They are a printable carrier of the Star Gate edge alphabet — not a cipher for natural language.

Screenshots of long symbol rows are **specimen pages** (font tests), not encoded sentences.

## Core symbolic keys (most important)

| Key | Code | Visual (from bitmap)        | Star Gate | Degree | Role |
|-----|------|-----------------------------|-----------|--------|------|
| `"` | 34   | sparse horizontal stripes   | `·` / `—` | **D1** | weak / draft / open link |
| `(` | 40   | interrupted stripes         | `∠`      | **D2** | angle / local turn |
| `'` | 39   | diamond lattice             | `◇`      | **D2** | stable bilateral link |
| `*` | 42   | quarter tiles               | `△` / tile| **D2** | minimal assemble unit |
| `&` | 38   | frame with windows          | `□̸`     | **D2** | shared / overlap |
| `$` | 36   | mirrored halves             | lever     | **D2–D3** | C ↔ I lever balance |
| `)` | 41   | solid split blocks          | `✕`      | **D3** | conflict / hard cut |
| `%` | 37   | dense bars                  | dense     | **D3** | high saturation |
| `#` | 35   | full checker / grid         | `⬡`-like | **D3** | hyper-group / max saturation |

## Digits `0`–`9` (codes 48–57)

Geometric motifs (small marks, corners, partial frames). Suggested use:

| Key | Suggested role |
|-----|----------------|
| `0` | empty / null node marker |
| `1`–`9` | lightweight node-type or weight hints in M2M lines |

Not required for core degree logic.

## Letters `A`–`Z` / `a`–`z`

In this font they are **abstract shapes** (frames, diagonals, arrows built from pixels), not Latin letterforms.

Recommended convention for AIL / UI:

| Keys | Use |
|------|-----|
| `A`–`Z` | Node type labels when font is active (e.g. T≈Transfer, R≈Refund) |
| `a`–`z` | Optional modifiers or secondary labels |

Degree is **not** taken from letter keys; degree comes from the symbolic keys above or from computed saturation.

## Four axes of an edge

```text
1. Star Gate symbol   — semantic character
2. EdgeKind           — Control / Data / Ownership / …
3. ConnectivityDegree — D1 / D2 / D3
4. glyph key/code     — keyboard char or font id for UI / M2M
```

## Specimen vs meaningful line

**Specimen (what the screenshots show):**
```text
ABCDEF...  "#$%&'()*  0123...
```
Just the catalogue of glyphs.

**Meaningful M2M line:**
```text
Transfer " CheckBalance
Transfer ) Refund
Refund ' Balance
PaymentsModule # Transfer Refund CheckBalance
```

## Solan3 vs Solan4

| Font | Emphasis |
|------|----------|
| **Solan4** | contour / linear edge alphabet |
| **Solan3** | same alphabet + denser filled glyphs (strong D3 textures) |

Use Solan4 for relation lines; Solan3 when you need heavy saturation/hyper textures in UI.

## VizGraph hint

| Degree | Prefer keys | Style |
|--------|-------------|-------|
| D1 | `"` | dashed, thin |
| D2 | `' * & ( $` | solid |
| D3 | `) % #` | bold, thick |

## Related docs

- `docs/visual-edges.md`
- `docs/connectivity-degree.md`
- `docs/lever-balance.md`
- `examples/m2m_glyph_line.md`
