# Edge Rules — Allowed Connections between VertexKinds

## VertexKind

- `Domain`   — hexagon vertex (Devices, Payments…)
- `Module`   — hyper-edge module
- `Function` — AIL function
- `Entity`   — concrete entity (Phone-X1, User, Order)
- `Detail`   — field of an entity
- `Article`  — content article / review

## Matrix of Allowed Edges

Legend:
- `+` = allowed
- `~` = allowed with restrictions
- `–` = forbidden

| From \ To       | Domain | Module | Function | Entity | Detail | Article |
|-----------------|--------|--------|----------|--------|--------|---------|
| **Domain**      | ◇ —   | → ⬡   | –        | –      | –      | –       |
| **Module**      | –      | ◇     | → — ▲   | –      | –      | –       |
| **Function**    | –      | –      | → — ▲ ✕ □̸ | → △ | – | – |
| **Entity**      | –      | –      | → △     | ◇ — △ | —     | △ □̸  |
| **Detail**      | –      | –      | →        | –      | —     | –       |
| **Article**     | –      | –      | –        | □̸ △ | –     | ◇ —   |

## Important Rules

### Function ↔ Function
Allowed almost all types:
- `→` sequential / data flow
- `—` simple dependency
- `▲` mutating effect
- `✕` conflict (Transfer ✕ Refund)
- `□̸` shared access

### Function → Entity
- `→` function works with entity
- `△` function influences entity

### Entity → Detail
- Only simple link `—`

### Article ↔ Entity
- `□̸` shared information
- `△` influence on perception

### Detail → Function
- `→` (example: Price → Transfer.amount)

## Hyper-edge (`⬡`) Rules

| Hyper-edge type     | Can contain                |
|---------------------|----------------------------|
| Module              | Function                   |
| Devices Group       | Entity + Detail            |
| Content Group       | Article + Entity           |
| Mixed (experimental)| Function + Entity (careful)|
