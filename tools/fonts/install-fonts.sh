#!/usr/bin/env bash
# Optional helper: if you keep originals elsewhere, copy them here.
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "Place myFront4Solan4.ttf and myFront4Solan3.ttf into:"
echo "  $DIR"
echo
if [[ -f "$DIR/myFront4Solan4.ttf" && -f "$DIR/myFront4Solan3.ttf" ]]; then
  ls -la "$DIR"/*.ttf
  echo "OK — fonts present. Run: cd tools && python3 -m http.server 8765"
else
  echo "Missing TTF files. Copy them into tools/fonts/ then re-run."
  exit 1
fi
