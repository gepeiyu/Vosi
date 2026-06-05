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
echo "bundled models from $SRC → $DEST"
