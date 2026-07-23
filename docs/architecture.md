# Architecture

## Layers

```text
8. Visualization (VizGraph)
7. .ail language
6. Analyzers (Ownership, Contracts, Edge Rules)
5. Hot-Swap + Lineage
4. Shard Actor
3. Interpreter
2. Binary AST
1. Core structures
```

## Core Ideas

### 1. Binary AST
Code is not text. Code is a graph of operations (`Op`).

### 2. Ownership
Every value has exactly one owner at any moment.
States: `Owned`, `Borrowed`, `BorrowedMut`, `Consumed`.

### 3. Hot-Swap with Weakening Check
Modules can be replaced only if contracts are not weakened.
Every successful swap creates a new generation (lineage).

### 4. Form Layer vs Semantic Layer
- **Continuum** stabilizes Form (projects, files, environment)
- **AIL Runtime** stabilizes Meaning (semantics, ownership, contracts)

### 5. GERMES Hexagon
6 domains arranged as projection of a cube along its space diagonal.
Center = coincidence of full Yin and full Yang.

### 6. Star Gate
Visual language of edges that describes the *character* of relations.

### 7. Code + Content as One Graph
Technical functions and content entities live in the same graph
and communicate through typed edges.
