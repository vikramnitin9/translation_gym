#!/usr/bin/env bash

set -euo pipefail

if [ $# -lt 1 ]; then
  echo "Usage: $0 <dataset_name>" >&2
  exit 1
fi
DATASET="$1"


mkdir -p output
SUFFIX="$(openssl rand -base64 32 | tr -dc '[:alnum:]' | head -c 10)"
HOST_OUT="output/${DATASET}_pruned_${SUFFIX}"
mkdir -p "$HOST_OUT"    
rm -rf "$HOST_OUT"

DOCKER_SOCKET="$(docker context inspect \
                   | grep '"Host"' | head -n1 \
                   | sed -E 's/.*"Host": *"unix:\/\/([^"]+)".*/\1/')"


docker run -it --rm \
  -u 0:0 \
  -v "$PWD/output":/app/output \
  -v "$PWD/translation_gym":/app/translation_gym \
  -v "$PWD/main.py":/app/main.py \
  -v "$PWD/data":/app/data \
  -v "$DOCKER_SOCKET":/var/run/docker.sock \
  translation_gym:latest \
  /bin/bash -c "python /app/main.py \
                   --dataset $DATASET \
                   --output_dir /app/$HOST_OUT \
                   --prune --verbose"
