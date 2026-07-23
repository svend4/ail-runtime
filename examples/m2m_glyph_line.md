# M2M line using Star Gate glyphs

Compact machine-to-machine (and human-readable with the font) description of relations.

## Example: PaymentsModule

```text
Transfer " CheckBalance
Transfer ) Refund
Refund ' Balance
PaymentsModule # Transfer Refund CheckBalance
```

Interpretation:

| Line | Meaning |
|------|--------|
| `Transfer " CheckBalance` | weak/open data link (D1) |
| `Transfer ) Refund` | hard conflict (D3) |
| `Refund ' Balance` | stable bilateral ownership-style link (D2) |
| `PaymentsModule # …` | hyper / group containing the three functions (D3) |

## Parse sketch

```text
line  := Node Glyph Node
Glyph := " | ( | ' | * | & | $ | ) | % | #

" → D1 open
( → D2 angle
' → D2 diamond
* → D2 tile
& → D2 shared
$ → lever marker (not always an edge)
) → D3 conflict
% → D3 dense
# → D3 hyper
```

After parse: create edges, run `saturation` / `assign_degree`, update `LeverBalance`.

## Why useful

- Shorter than JSON for M2M gossip of graph deltas
- Same alphabet as UI when `myFront4Solan4` is installed
- Maps 1:1 onto existing ownership and hot-swap rules
