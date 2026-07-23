# Example: Commerce Graph (Hexagon + Code + Content)

## Top-level Hexagon

```text
1. Devices      --+   →
2. Payments     -+-   ◇
3. Users        +--   △
4. Logistics    -++   ▲
5. Content      +-+   □̸
6. Analytics    ++-   —

Center: ✕  (Core)
```

## Payments Module

```text
⬡ PaymentsModule

  Transfer       →
  CheckBalance   —
  Refund         ▲

  Transfer  ✕  Refund
```

## Devices + Content

```text
⬡ Devices

  Entity: Phone-X1
    — Price = 799
    — Memory = 256GB
    △ Article "Обзор Phone-X1"

  Entity: Buds-Pro
    — Price = 199
```

## Cross links

```text
Phone-X1.Price     →  Transfer
Phone-X1           →  CheckBalance
Article            □̸  Phone-X1
Transfer           ✕  Refund
```

## Hierarchy (General → Specific)

```text
Core
 → Domains (Hexagon)
   → Modules / Groups (⬡)
     → Functions / Entities
       → Binary AST / Details
```
