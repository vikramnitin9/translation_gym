#!/usr/bin/env bash
# -----------------------------------------------------------
# run_C_metrics.sh
# -----------------------------------------------------------
# Usage:
#   ./run_C_metrics.sh <c-source-dir> [covered.txt]
#
#   <c-source-dir>   relative directory containing *.c files
#   [covered.txt]    optional file with one function name per line
#
# The script:
#   1) Re-creates compile_commands.json with Bear
#   2) Builds tools/C_metrics (cmake + make) inside the container
#   3) Runs the freshly-built C_metrics on every *.c file
# -----------------------------------------------------------
set -euo pipefail

if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "Usage: $0 <c-source-dir> [covered.txt]"
  exit 1
fi
SRC_DIR=$1
COVERED_LIST=${2:-}

IMAGE=translation_gym:latest

DOCKER_SOCKET=$(docker context inspect 2>/dev/null \
                  | grep -m1 '"Host"' \
                  | sed -E 's/.*"unix:\/\/([^"]+)".*/\1/')
[[ -S $DOCKER_SOCKET ]] || { echo "❌  Docker socket not found"; exit 1; }

# 1. Generate compile_commands.json
echo "🔨  Generating compile_commands.json with Bear …"
docker run --rm -u 0:0 \
  -v "$PWD":/project \
  -v "$DOCKER_SOCKET":/var/run/docker.sock \
  "$IMAGE" \
  bash -lc "
    cd /project/$SRC_DIR
    rm -f /project/compile_commands.json
    bear --cdb /project/compile_commands.json make
  "

# 2. Run C_metrics on every .c file

echo "📊  Running C_metrics on each *.c in '$SRC_DIR' …"
docker run --rm -u 0:0 \
  -v "$PWD":/project \
  "$IMAGE" \
  bash -lc '
    set -e
    BIN=/project/tools/C_metrics/build/C_metrics
    cd /project/'"$SRC_DIR"'

    for f in *.c; do
      echo
      echo "--- metrics for $f ---"
      if [[ -n "'"$COVERED_LIST"'" ]]; then
        "$BIN" --covered-functions /project/'"$COVERED_LIST"' "$f"
      else
        "$BIN" "$f"
      fi
    done
  '

echo "✅  Done."
