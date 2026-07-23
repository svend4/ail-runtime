# Star Gate Virtual Keyboard & Scientific Calculator

## Files

| File | Role |
|------|------|
| `tools/stargate-calculator.html` | **Main UI** — calculator-style board |
| `tools/stargate-keyboard.html` | Simpler earlier prototype |

Open `stargate-calculator.html` in a browser (local file is enough).

## What it includes

1. **22 macro keys (F1–F22)** on top — large, like a scientific calculator function row  
   Lnk, Rsk, Bal, Rei, Swp, RAG, Path, Health, Sat, Deg, Viz, Hyp, …

2. **Star Gate alphabet row** — `" ( ' * & $ ) % #` coloured by D1/D2/D3

3. **Dual keys** — each letter shows **Latin ↔ glyph** correspondence  
   Modes: Dual / Latin only / Glyph only

4. **Layouts**
   - **EN** QWERTY
   - **RU (РФ)** ЙЦУКЕН
   - **Star Gate only** node shortcuts

5. **Display** like a calculator screen + last pair + degree readout

## Correspondence idea

Letters are not random: the dual map teaches which glyph cluster a key is paired with (for training and M2M typing). Actual graph semantics always come from the **glyph** (`" ' ) # …`), not from the Latin letter alone.

## RAG / graph macros

Same policy as `docs/rag-macros.md`:

- prefer D2 paths
- D1 = fallback
- D3 `)` = do not cross
- `#` = atomic hyper cluster

## Optional next steps

- Load `myFront4Solan4.ttf` via `@font-face` so keys render real glyphs
- Wire macros to a local WASM/CLI backend calling `connectivity` APIs
- Add SPN (e.g. Spanish) layout the same way as RU if needed
