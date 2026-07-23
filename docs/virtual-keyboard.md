# Star Gate Virtual Keyboard

Virtual input surface for the geometric alphabet (myFront4Solan3/4).
Not only key → glyph, but key → **function / cluster / macro** for graph & RAG work.

## Layout (logical)

```text
┌──────────────────────────────────────┐
│  D1 ZONE          D2 ZONE              D3 ZONE      │
│  [ " ] open       [ ( ] angle          [ ) ] conflict│
│                   [ ' ] diamond        [ % ] dense  │
│                   [ * ] tile           [ # ] hyper  │
│                   [ & ] shared                       │
│                   [ $ ] lever                        │
│                                                      │
│  NODES: [T]ransfer [R]efund [B]alance [M]odule ... │
│  MACROS: [Lnk] [Rsk] [Bal] [Rei] [RAG] [Swp]        │
└──────────────────────────────────────┘
```

## Key → meaning

| Key | Glyph role | Degree | Cluster / function |
|-----|------------|--------|--------------------|
| `"` | open stripe | D1 | weak link, draft edge, low-priority RAG hop |
| `(` | angle | D2 | turn / transform / pipeline step |
| `'` | diamond | D2 | stable bilateral relation |
| `*` | tile | D2 | compose unit, small cycle |
| `&` | shared window | D2 | shared borrow / multi-read |
| `$` | lever | D2–D3 | show C↔I balance / health |
| `)` | split | D3 | conflict, exclude, hard constraint |
| `%` | dense | D3 | high saturation cluster |
| `#` | grid | D3 | hyper-edge / atomic group |

## Node label keys (examples)

| Key | Suggested node type |
|-----|---------------------|
| T | Transfer / Task |
| R | Refund / Resource |
| B | Balance / Buffer |
| M | Module |
| E | Entity / Product |
| Q | Query (RAG) |
| C | Chunk (RAG) |
| D | Document |

## Macros (graph + RAG)

| Macro | Expands to / does |
|-------|-------------------|
| `Lnk a g b` | create edge a—g—b, recompute saturation |
| `Rsk` | print hot-swap / connectivity risk for selection |
| `Bal` | compute LeverBalance + marginality |
| `Rei e +δ` | reinforce_edge (pheromone) |
| `Swp` | propose hot-swap candidate with risk check |
| `RAG q` | retrieve along D2 first, D1 fallback, respect D3 excludes |
| `Path a b` | path preferring D2 edges |
| `Health` | dump RuntimeMetrics + lever |

## RAG policy with the alphabet

1. Prefer paths whose edges are **D2** (`' * & (`).
2. Use **D1** (`"`) only as weak bridges.
3. Never cross **D3 conflict** `)` unless explicitly allowed.
4. Treat `#` groups as atomic retrieval units (whole hyper-node).
5. `$` triggers a health check before large retrieval batches.

## Implementation options

1. **HTML virtual keyboard** — `tools/stargate-keyboard.html` (buttons insert glyph + tooltip).
2. **Editor snippet pack** — each macro as snippet in IDE.
3. **CLI** — parse M2M lines and macro commands into Graph API calls.

The HTML prototype is the fastest way to type and learn the alphabet.
