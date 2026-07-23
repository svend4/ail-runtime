# Hot-Swap + Lineage

## Two-Stage Check

When a new module version arrives:

```text
New BinaryModule
        |
        v
+-------------------+
|  Stage 1: Quick   |   cheap structural filter
+-------------------+
        |
   passed? -- No --> Reject
        |
       Yes
        |
        v
+-------------------+
|  Stage 2: Full   |   ownership + contracts + weakening
+-------------------+
        |
   passed? -- No --> Reject
        |
       Yes
        |
        v
  Atomic Hot-Swap + new generation
```

### Stage 1 — Quick Structural Filter

- All public functions of the old module still exist
- Number of contracts does not decrease
- Resource limits are not relaxed
- Basic size / complexity heuristics

### Stage 2 — Full Semantic Check

- `contracts_not_weakened`
- Ownership analysis of the new module
- Optional SMT / property checks on critical paths

## Lineage Fields

Every module carries:

```rust
content_hash: String,
parent_hash: Option<String>,
generation: u32,
origin: ModuleOrigin,   // Manual | Healed | Loaded
```

## Example Lineage

```text
gen 0  (Manual)          hash: aaa
  |
  +--> gen 1  (Healed)   hash: bbb   parent: aaa
         |
         +--> gen 2  (Healed) hash: ccc   parent: bbb
```

## Origin

```rust
enum ModuleOrigin {
    Manual,
    Healed {
        reason: String,
        from_generation: u32,
    },
    Loaded,
}
```

## Relation to Continuum

Continuum uses generation lineage for plans and environment repairs.
AIL Runtime uses the same idea inside the Semantic Layer for modules and important entities.
