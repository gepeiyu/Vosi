#!/usr/bin/env bash
# Copy a manually downloaded sherpa punctuation model into models/dev/.
set -euo pipefail

SRC="${1:-$HOME/Downloads/model.onnx}"
DEST="$(cd "$(dirname "$0")/.." && pwd)/models/dev/punctuation/model.onnx"

if [[ ! -f "$SRC" ]]; then
  echo "Source not found: $SRC" >&2
  exit 1
fi

SIZE=$(stat -f%z "$SRC" 2>/dev/null || stat -c%s "$SRC")
if (( SIZE < 250000000 )); then
  echo "Warning: $SRC is only $(( SIZE / 1024 / 1024 ))MB; expected ~280MB" >&2
fi

mkdir -p "$(dirname "$DEST")"
cp "$SRC" "$DEST"
rm -f "$(dirname "$DEST")/archive.tar.bz2"
echo "Installed → $DEST ($(ls -lh "$DEST" | awk '{print $5}'))"
