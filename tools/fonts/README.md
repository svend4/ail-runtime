# Solan fonts for Star Gate keyboard

## Quick setup

Copy your font files into this folder with exact names:

```text
tools/fonts/myFront4Solan4.ttf
tools/fonts/myFront4Solan3.ttf
```

(You already have these as `myFront4Solan4.ttf` / `myFront4Solan3.ttf` from the project attachments.)

Then serve the tools folder:

```bash
cd tools
python3 -m http.server 8765
```

Open: http://localhost:8765/stargate-calculator.html

In the toolbar switch **Solan4** / **Solan3**.

## Why a local server?

Browsers often block `@font-face` from `file://` URLs. A one-line HTTP server fixes that.

## What the calculator uses

```css
@font-face {
  font-family: "Solan4";
  src: url("fonts/myFront4Solan4.ttf") format("truetype");
}
@font-face {
  font-family: "Solan3";
  src: url("fonts/myFront4Solan3.ttf") format("truetype");
}
```

Screen + glyph sublabels on keys render with these faces.
