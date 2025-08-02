#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────
#  prune_only.sh  –  Prune an already-translated project
#
#  Usage:   ./prune_only.sh <translated_output_dir>
#
#  Example: ./prune_only.sh output/cat_9tNXNBmRIy
#           ^ infers DATASET = "cat"
#
#  Notes:
#    • The folder must already contain /target (Rust) and /source (C) trees.
#    • Pruning is done in-place; nothing is copied or re-translated.
# ──────────────────────────────────────────────────────────────
set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <translated_output_dir>" >&2
  exit 1
fi
OUT_DIR_RAW="$1"

# ── convert to ABSOLUTE host path for Docker bind-mount ────────────────
if [[ "$OUT_DIR_RAW" != /* ]]; then
  OUT_DIR="$(cd "$(dirname "$OUT_DIR_RAW")" && pwd)/$(basename "$OUT_DIR_RAW")"
else
  OUT_DIR="$OUT_DIR_RAW"
fi

if [ ! -d "$OUT_DIR" ]; then
  echo "❌  '$OUT_DIR' is not a directory or does not exist." >&2
  exit 1
fi

# ── infer dataset name (basename before first "_") ─────────────────────
DATASET="$(basename "$OUT_DIR" | cut -d'_' -f1)"
if [ -z "$DATASET" ]; then
  echo "❌  Could not infer dataset name from '$OUT_DIR'." >&2
  echo "    Make sure the folder name looks like <dataset>_<hash>/." >&2
  exit 1
fi
echo "📦  Dataset inferred as: $DATASET"

# ── locate Docker socket for rootless setups (same trick as run.sh) ───
DOCKER_SOCKET="$(docker context inspect | \
  grep '\"Host\"' | head -n1 | sed -E 's/.*"Host": *"unix:\/\/([^"]+)".*/\1/')"

# ── launch prune-only pass inside container ────────────────────────────
docker run -it --rm \
  -u 0:0 \
  -v "$OUT_DIR":/app/output \
  -v "$PWD/data":/app/data \
  -v "$PWD/translation_gym":/app/translation_gym \
  -v "$PWD/main.py":/app/main.py \
  -v "$DOCKER_SOCKET":/var/run/docker.sock \
  translation_gym:latest \
  /bin/bash -c "python /app/main.py \
      --dataset ${DATASET} \
      --output_dir /app/output \
      --prune_only --verbose"
