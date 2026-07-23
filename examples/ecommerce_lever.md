# E-commerce lever example

Small product graph:

```text
Phone-X1  —  Price
Phone-X1  →  Similar:Phone-Y2
Phone-X1  ◇  ReviewCluster
Phone-Y2  —  Price
ReviewCluster  ✕  FakeReviewBucket   (conflict)
```

## Intended degrees

| Edge                         | Expected zone | Why                          |
|------------------------------|---------------|------------------------------|
| Phone-X1 — Price             | D1 / low D2   | weak attribute link          |
| Phone-X1 → Similar           | D2            | navigation / recommendation  |
| Phone-X1 ◇ ReviewCluster     | D2            | stable but flexible          |
| ReviewCluster ✕ FakeReview   | D3            | hard exclusion               |

## Balancing actions

- If many products have only D1 attribute links → catalogue feels sparse:
  `reinforce` similarity edges (raise weight / saturation toward D2).

- If conflict and hyper edges dominate → catalogue feels rigid:
  split categories or demote some D3 links.

- Target: `LeverBalance` not tilted, `marginality_ratio` near 0.5.

## Code sketch

```rust
let balance = LeverBalance::from_degrees(graph.edges.iter().map(|e| &e.degree));
balance.print();
if balance.tilted {
    // recommend reinforce on strategic D1 edges or decay on excess D3
}
```
