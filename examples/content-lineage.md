# Content Lineage Example (Phone-X1)

Content entities follow the same lineage model as modules.

## Evolution of Phone-X1

```text
gen 0  (Created)
  hash: 111aaa
  Price  = 849
  Memory = 128GB
       |
       v
gen 1  (Updated)
  hash: 222bbb
  parent: 111aaa
  reason: "Price drop + memory upgrade"
  Price  = 799
  Memory = 256GB
       |
       v
gen 2  (Updated)
  hash: 333ccc
  parent: 222bbb
  reason: "Added new color variant"
  Price  = 799
  Memory = 256GB
  Color  = Black | Silver
```

## Link with Code Lineage

```text
Phone-X1 (gen 2)  →  Transfer (from PaymentsModule gen 2)
```

Both code and content carry:

- `content_hash`
- `parent_hash`
- `generation`
- `origin`

This allows reproducible combinations of a specific product version with a specific module version.
