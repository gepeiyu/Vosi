#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
mkdir -p "$ROOT/models/dev"

download_and_verify() {
  local url="$1" dest="$2"
  mkdir -p "$dest"
  local archive="$dest/archive"
  echo "Downloading $url ..."
  curl -L "$url" -o "$archive"
  echo "SHA256: $(shasum -a 256 "$archive" | awk '{print $1}')"
  case "$archive" in
    *.tar.bz2) tar -xjf "$archive" -C "$dest" && rm -f "$archive" ;;
    *.onnx) mv "$archive" "$dest/model.onnx" ;;
    *) echo "Unknown archive type: $archive" >&2; exit 1 ;;
  esac
}

download_and_verify \
  "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-2024-03-09.tar.bz2" \
  "$ROOT/models/dev/paraformer-zh"

download_and_verify \
  "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx" \
  "$ROOT/models/dev/vad"

download_and_verify \
  "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab471067-large.tar.bz2" \
  "$ROOT/models/dev/punctuation"

echo "Done. Copy SHA256 values into models/manifest.json"
