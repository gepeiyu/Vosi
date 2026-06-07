#!/usr/bin/env bash
# Download sherpa-onnx packaged models for local development.
#
# Mirrors (VOSI_MODEL_MIRROR):
#   auto      — 有代理时 GitHub/HuggingFace，否则 hf-mirror → modelscope(VAD) → github
#   github    — GitHub Releases + huggingface.co（需代理时设 VOSI_PROXY）
#   hf-mirror — HuggingFace 国内镜像（csukuangfj 预打包 ONNX）
#   modelscope — 仅 VAD；ASR/标点须 sherpa 格式，魔搭 FunASR ONNX 不兼容
#
# 代理（下载 GitHub / huggingface.co）:
#   export VOSI_PROXY=http://127.0.0.1:7890
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MIRROR="${VOSI_MODEL_MIRROR:-auto}"
DEST_ROOT="$ROOT/models/dev"
PROXY="${VOSI_PROXY:-${HTTPS_PROXY:-${https_proxy:-${HTTP_PROXY:-${http_proxy:-}}}}}"

log() { echo "[download-models] $*"; }

curl_fetch() {
  local url="$1" out="$2"
  local -a args=(
    -L --http1.1 --retry 5 --retry-delay 3
    --connect-timeout 30 --max-time 0 -C - -o "$out"
  )
  if [[ -n "$PROXY" ]]; then
    args+=(-x "$PROXY")
  fi
  curl "${args[@]}" "$url"
}

setup_proxy_env() {
  if [[ -n "$PROXY" ]]; then
    export http_proxy="$PROXY" https_proxy="$PROXY"
    export HTTP_PROXY="$PROXY" HTTPS_PROXY="$PROXY"
    log "proxy=$PROXY"
  fi
}

ensure_modelscope_cli() {
  if command -v modelscope >/dev/null 2>&1; then
    return 0
  fi
  if ! command -v python3 >/dev/null 2>&1; then
    return 1
  fi
  log "installing modelscope CLI (pip)..."
  python3 -m pip install -q -U modelscope
  command -v modelscope >/dev/null 2>&1
}

download_modelscope() {
  local model_id="$1" dest="$2"
  local cache="$ROOT/.cache/modelscope-download/$model_id"
  mkdir -p "$dest" "$cache"
  log "ModelScope: $model_id → $dest"
  modelscope download --model "$model_id" --local_dir "$cache"
  rm -rf "${dest:?}/"*
  cp -R "$cache"/. "$dest"/
}

hf_base_url() {
  if [[ -n "$PROXY" ]] || [[ "$MIRROR" == "github" ]]; then
    echo "https://huggingface.co"
  else
    echo "https://hf-mirror.com"
  fi
}

download_hf_file() {
  local repo="$1" file="$2" out="$3"
  local base
  base="$(hf_base_url)"
  local url="${base}/${repo}/resolve/main/${file}"
  log "HF: $url"
  mkdir -p "$(dirname "$out")"
  curl_fetch "$url" "$out"
}

download_github() {
  local url="$1" dest="$2"
  mkdir -p "$dest"
  local archive
  if [[ "$url" == *.onnx ]]; then
    archive="$dest/model.onnx"
  else
    archive="$dest/archive"
  fi
  log "GitHub: $url"
  curl_fetch "$url" "$archive"
  echo "SHA256: $(shasum -a 256 "$archive" | awk '{print $1}')"
  case "$archive" in
    *.tar.bz2)
      tar -xjf "$archive" -C "$dest"
      rm -f "$archive"
      local sub
      sub="$(find "$dest" -mindepth 1 -maxdepth 1 -type d ! -name '.*' | head -1)"
      if [[ -n "$sub" && "$sub" != "$dest" ]]; then
        shopt -s dotglob
        mv "$sub"/* "$dest"/
        shopt -u dotglob
        rmdir "$sub" 2>/dev/null || true
      fi
      ;;
    *.onnx) ;;
    *) echo "Unknown archive type: $archive" >&2; return 1 ;;
  esac
}

try_mirror() {
  local name="$1"
  shift
  "$@" && return 0
  log "$name failed"
  return 1
}

use_foreign_first() {
  [[ -n "$PROXY" ]] || [[ "$MIRROR" == "github" ]]
}

download_sense_voice() {
  local dest="$DEST_ROOT/sense-voice"
  mkdir -p "$dest"

  if use_foreign_first || [[ "$MIRROR" == "hf-mirror" ]] || [[ "$MIRROR" == "auto" ]]; then
    if try_mirror hf \
      download_hf_file "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17" "model.int8.onnx" "$dest/model.int8.onnx" \
      && download_hf_file "csukuangfj/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17" "tokens.txt" "$dest/tokens.txt"; then
      return 0
    fi
    [[ "$MIRROR" == "hf-mirror" ]] && return 1
  fi

  if [[ "$MIRROR" == "github" ]] || [[ "$MIRROR" == "auto" ]]; then
    try_mirror github download_github \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-sense-voice-zh-en-ja-ko-yue-2024-07-17.tar.bz2" \
      "$dest" && return 0
    [[ "$MIRROR" == "github" ]] && return 1
  fi

  return 1
}

# sherpa 预打包 INT8 小模型（与 sherpa-onnx 兼容；魔搭 FunASR ONNX 缺 metadata 不可用）
download_paraformer() {
  local dest="$DEST_ROOT/paraformer-zh"
  mkdir -p "$dest"

  if use_foreign_first || [[ "$MIRROR" == "hf-mirror" ]] || [[ "$MIRROR" == "auto" ]]; then
    if try_mirror hf \
      download_hf_file "csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09" "model.int8.onnx" "$dest/model.int8.onnx" \
      && download_hf_file "csukuangfj/sherpa-onnx-paraformer-zh-small-2024-03-09" "tokens.txt" "$dest/tokens.txt"; then
      return 0
    fi
    [[ "$MIRROR" == "hf-mirror" ]] && return 1
  fi

  if [[ "$MIRROR" == "github" ]] || [[ "$MIRROR" == "auto" ]]; then
    try_mirror github download_github \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-paraformer-zh-small-2024-03-09.tar.bz2" \
      "$dest" && return 0
    [[ "$MIRROR" == "github" ]] && return 1
  fi

  return 1
}

download_vad() {
  local dest="$DEST_ROOT/vad"
  mkdir -p "$dest"

  # silero 仅 ~2MB，GitHub + 代理最快
  if use_foreign_first || [[ "$MIRROR" == "github" ]] || [[ "$MIRROR" == "auto" ]]; then
    if try_mirror github download_github \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx" \
      "$dest"; then
      return 0
    fi
  fi

  if [[ "$MIRROR" == "modelscope" ]] || [[ "$MIRROR" == "auto" ]]; then
    if ensure_modelscope_cli; then
      try_mirror modelscope download_modelscope \
        "iic/speech_fsmn_vad_zh-cn-16k-common-onnx" "$dest" && return 0
    fi
  fi

  download_github \
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx" \
    "$dest"
}

# sherpa 标点模型（魔搭 model_quant.onnx 不兼容）
download_punctuation() {
  local dest="$DEST_ROOT/punctuation"
  mkdir -p "$dest"
  # 去掉不完整的 model.onnx 或魔搭 quant 残留，避免误用
  rm -f "$dest/model.onnx" "$dest/model_quant.onnx"

  if use_foreign_first || [[ "$MIRROR" == "hf-mirror" ]] || [[ "$MIRROR" == "auto" ]]; then
    if try_mirror hf \
      download_hf_file "csukuangfj/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12" "model.onnx" "$dest/model.onnx"; then
      return 0
    fi
    [[ "$MIRROR" == "hf-mirror" ]] && return 1
  fi

  if [[ "$MIRROR" == "github" ]] || [[ "$MIRROR" == "auto" ]]; then
    try_mirror github download_github \
      "https://github.com/k2-fsa/sherpa-onnx/releases/download/punctuation-models/sherpa-onnx-punct-ct-transformer-zh-en-vocab272727-2024-04-12.tar.bz2" \
      "$dest" && return 0
    [[ "$MIRROR" == "github" ]] && return 1
  fi

  return 1
}

setup_proxy_env
mkdir -p "$DEST_ROOT"
log "mirror=$MIRROR dest=$DEST_ROOT"

download_sense_voice
download_paraformer   # 测试期保留备份
download_vad
download_punctuation

log "Done → $DEST_ROOT"
if [[ -z "$PROXY" ]]; then
  log "Tip: 国外源可设 export VOSI_PROXY=http://127.0.0.1:7890"
fi
