# How to read font specimen screenshots

If OpenOffice (or any editor) shows a long row of geometric signs while font `myFront4Solan4` / `Solan3` is selected:

1. This is almost always a **specimen** — keys typed in order to preview glyphs.
2. It is **not** a decoded natural-language sentence.
3. Classify marks by density:
   - open contours → D1
   - triangles / diamonds / windows → D2
   - crosses / solid grids / fills → D3
4. For real graph text, use the M2M pattern:

```text
NodeName  GLYPH  NodeName
```

with `GLYPH` one of: `" ( ' * & $ ) % #`

See `docs/font-stargate-map.md` and `examples/m2m_glyph_line.md`.
