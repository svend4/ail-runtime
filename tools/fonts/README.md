# Solan fonts for Star Gate keyboard

Place here:

- `myFront4Solan4.ttf`
- `myFront4Solan3.ttf`

Or run from repo root:

```bash
bash tools/fonts/install-fonts.sh
```

This decodes the `.b64` sidecars into `.ttf` files next to the HTML.

Then open `tools/stargate-calculator.html` in a browser (via a local static server if needed):

```bash
cd tools && python3 -m http.server 8765
# open http://localhost:8765/stargate-calculator.html
```
