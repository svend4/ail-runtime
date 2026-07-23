#!/usr/bin/env bash
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
base64 -d "$DIR/myFront4Solan4.ttf.b64" > "$DIR/myFront4Solan4.ttf"
base64 -d "$DIR/myFront4Solan3.ttf.b64" > "$DIR/myFront4Solan3.ttf"
echo "Installed:"
ls -la "$DIR"/*.ttf
