#!/usr/bin/env bash
# Regenerate every platform icon from the canonical SVG.
#
# Source of truth:  assets/icon/source.svg
# Outputs:
#   - apps/desktop-viewer/src-tauri/icons/{32x32,128x128,128x128@2x,icon.icns,icon.ico,icon.png}
#       via `tauri icon` so macOS / Windows / Linux desktop bundles pick
#       up the right format.
#   - apps/web-playground/public/icon.svg
#       served by Vite at /icon.svg, referenced from index.html as the
#       browser-tab favicon. The web bundle keeps the SVG vector form
#       so it stays sharp at every favicon size.
#
# Re-run after editing source.svg. Idempotent — safe to run repeatedly.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SOURCE_SVG="$REPO_ROOT/assets/icon/source.svg"

if [[ ! -f "$SOURCE_SVG" ]]; then
  echo "error: $SOURCE_SVG not found" >&2
  exit 1
fi

# Need a high-res PNG to feed `tauri icon`. resvg ships with the
# slideglance-png crate's dev tooling (`cargo install resvg`); fall back to
# `rsvg-convert` if resvg isn't on PATH.
TMP_PNG="$(mktemp -t pptx-icon-source).png"
trap 'rm -f "$TMP_PNG"' EXIT

if command -v resvg >/dev/null 2>&1; then
  resvg --width 1024 --height 1024 "$SOURCE_SVG" "$TMP_PNG"
elif command -v rsvg-convert >/dev/null 2>&1; then
  rsvg-convert -w 1024 -h 1024 "$SOURCE_SVG" -o "$TMP_PNG"
else
  echo "error: neither resvg nor rsvg-convert is on PATH" >&2
  exit 1
fi

# Desktop bundle icons. `tauri icon` wipes and rewrites the output
# directory (apps/desktop-viewer/src-tauri/icons by default).
echo "==> regenerating desktop bundle icons"
pnpm --filter @slideglance/desktop-viewer tauri icon "$TMP_PNG"

# Web playground favicon. The SVG ships verbatim — modern browsers
# render it sharp at every size. Older browsers fall back to the
# default and that's fine for a dev playground.
echo "==> syncing web playground favicon"
cp "$SOURCE_SVG" "$REPO_ROOT/apps/web-playground/public/icon.svg"

echo "done."
