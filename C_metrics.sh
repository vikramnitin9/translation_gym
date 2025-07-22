#!/usr/bin/env bash
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "Usage: $0 <relative-path-to-c-dir>   e.g.  ./C_metrics.sh data/toy"
  exit 1
fi
SRC_DIR=$1
IMAGE=translation_gym:latest

# rootless-docker socket (only Bear needs it)
DOCKER_SOCKET=$(docker context inspect | grep '"Host"' | head -1 \
                 | sed -E 's/.*"Host": *"unix:\/\/([^"]+)".*/\1/')
[[ -S $DOCKER_SOCKET ]] || { echo "❌  Docker socket not found"; exit 1; }

##############################################################################
# 1. Generate compile_commands.json
##############################################################################
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

##############################################################################
# 2. Run C_metrics (binary is on PATH) on every .c file
##############################################################################
echo "📊  Running C_metrics on each *.c in '$SRC_DIR' …"
docker run --rm -u 0:0 \
  -v "$PWD":/project \
  "$IMAGE" \
  bash -lc "
    cd /project/$SRC_DIR
    for f in *.c; do
      echo
      echo \"--- metrics for \$f ---\"
      C_metrics \"\$f\"
    done
  "

echo "✅  Done."
