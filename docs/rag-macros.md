# RAG macros with Star Gate alphabet

## Goal

Use the same edge alphabet for retrieval-augmented generation graphs.

## Edge policy during retrieval

| Degree | Glyph keys | RAG behaviour |
|--------|------------|---------------|
| D1 | `"` | weak bridge only if D2 path missing |
| D2 | `' * & (` | **preferred** retrieval paths |
| D3 `)` | conflict | do not cross unless forced |
| D3 `# %` | hyper / dense | treat as atomic cluster (return whole group) |

## Macros

```text
RAG <query>          — run retrieval with policy above
Path <a> <b>         — shortest / best path preferring D2
Lnk <a> <glyph> <b>  — assert a relation (updates graph used by RAG)
Bal                  — health of knowledge graph (lever + marginality)
Rei <edge> +<d>      — boost a useful path (pheromone)
```

## Example session

```text
Lnk PhoneX1 ' SimilarY2
Lnk PhoneX1 " Price
Lnk ReviewCluster ) FakeBucket
RAG battery life PhoneX1
Bal
```

Retrieval walks `'` and other D2 edges first; skips `)` conflicts; may use `"` only as fallback.

## Link to runtime

Macros are intended to call the same Graph / LeverBalance / reinforce APIs as the MVP (`connectivity`, `lever`, actor metrics).
