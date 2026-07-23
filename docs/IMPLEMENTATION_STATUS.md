# Implementation Status

## Completed 5-step plan

| Step | Description                                      | Status |
|------|--------------------------------------------------|--------|
| 1    | Relation Graph inside ShardActor                 | ✅     |
| 2    | Hot-swap consults connectivity risk              | ✅     |
| 3    | Weighted saturation (pheromone / weight)         | ✅     |
| 4    | Marginality as runtime metric after each swap    | ✅     |
| 5    | VizGraph styles edges by D1/D2/D3                | ✅     |

## Core Runtime
- Binary AST + interpreter
- Actor / ShardActor
- Ownership (move / consume)
- Hot-Swap + lineage

## Connectivity Layer (`mvp/src/connectivity.rs`)
- Graph, saturation (weighted), D1/D2/D3
- Borrow helpers, hot_swap_risk, marginality_ratio
- `reinforce_edge` for pheromone updates

## Actor + Graph (`mvp/src/actor_with_graph.rs`)
- Owns module + relations + metrics_log
- Risk-aware `hot_swap`
- RuntimeMetrics snapshot after changes

## VizGraph (`mvp/src/vizgraph.rs`)
- `VizScene` with nodes/edges
- Colour, width, line style from degree + weight
- Circular layout + terminal dump

## Docs
- Full specification set under `docs/`
- Step notes under `docs/steps/`

## Suggested next directions (beyond the 5 steps)
1. Serialize `VizScene` to JSON for a web canvas
2. Force-directed layout instead of circle
3. Feed Binary AST data-flow edges into the same relation graph
4. Automatic pheromone decay over time
5. Merge `ShardActorWithGraph` into the main binary demo path
