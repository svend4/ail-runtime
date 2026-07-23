# Star Gate Virtual Keyboard & Scientific Calculator

## Main UI

**`tools/stargate-calculator.html`**

Features:

- Solan4 / Solan3 fonts (real glyphs on keys and screen)
- 22 macro keys F1–F22
- EN + RU (РФ) layouts
- Dual letter ↔ glyph keys
- Star Gate alphabet row coloured by D1/D2/D3
- Calculator-style display

## Install fonts

```bash
bash tools/fonts/install-fonts.sh
cd tools && python3 -m http.server 8765
```

Open `http://localhost:8765/stargate-calculator.html`.

(Browsers often block `file://` font loads; a tiny local server is reliable.)

## Font switch

In the toolbar: **Solan4** / **Solan3** — same alphabet, micro differences on a few letter glyphs.

## Related

- `docs/font-stargate-map.md`
- `docs/rag-macros.md`
- `examples/m2m_glyph_line.md`
