#!/usr/bin/env bash
# Copy dev models into the Tauri bundle resources directory before release build.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="${1:-$ROOT/models/dev}"
DEST="$ROOT/src-tauri/models/bundled"

mkdir -p "$DEST"
rm -rf "${DEST:?}/"*

if [[ ! -d "$SRC" ]] || [[ -z "$(ls -A "$SRC" 2>/dev/null)" ]]; then
  echo "warning: no models in $SRC — bundle will ship without ASR weights" >&2
  touch "$DEST/.gitkeep"
  exit 0
fi

cp -R "$SRC"/. "$DEST"/

# Runtime-only: SenseVoice INT8 + tokens (skip fp32 model.onnx and test assets).
sv="$DEST/sense-voice"
if [[ -d "$sv" ]]; then
  rm -f "$sv/model.onnx" "$sv/export-onnx.py"
  rm -rf "$sv/test_wavs"
fi

echo "bundled models from $SRC → $DEST"
