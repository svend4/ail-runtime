# Comparison with Existing Systems

## Summary Table

| System              | Ownership | Actors | Hot-swap / Evolution | Contracts | Maturity |
|---------------------|-----------|--------|----------------------|-----------|----------|
| **AIL Runtime**     | Yes       | Yes    | Yes (weakening)      | Yes       | Early    |
| Move                | Excellent | No     | No                   | Yes       | High     |
| Erlang / Akka       | No        | Excellent | Yes                | Weak      | Very High|
| Orleans             | No        | Excellent | Partial              | Weak      | High     |
| WASM Component Model| Weak      | No     | No                   | Via interfaces | Growing |
| Continuum (meta1)   | No        | No     | Yes (Repair Cascade) | Assertions| Early    |

## Detailed Notes

### Move Language
- Closest relative in **ownership** and resource semantics.
- Strong linear types.
- No actor model, no built-in hot-swap with contract weakening.

### Akka / Erlang
- Excellent actors and distribution.
- Hot code swapping exists (especially Erlang).
- No fine-grained ownership of data.

### Orleans
- Virtual actors, great lifecycle management.
- We can learn distribution patterns from it.
- Ownership and formal contracts are weaker.

### Continuum (meta1)
- Focuses on **Form Layer**: plans, environment, dependency drift, lineage of generations.
- AIL Runtime focuses on **Semantic Layer**: meaning, ownership, contracts inside the runtime.
- They complement each other well:
  - Continuum stabilizes the outside world.
  - AIL Runtime stabilizes the meaning of the code.

### WASM Component Model
- Strong isolation and composition.
- Different goal (portability).
- Almost no ownership at business-object level.

## Unique Position of AIL Runtime

Most systems are strong in **one** dimension.
AIL Runtime tries to combine three at once:

1. Ownership (inspired by Move / Rust)
2. Actor isolation
3. Safe evolution via weakening-check + lineage
