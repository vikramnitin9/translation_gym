#!/usr/bin/env bash
set -euo pipefail

docker run --rm -it \
  -u 0:0 \
  -v "$PWD":/project \
  translation_gym:latest \
  /bin/bash -lc "cd /project && python3 compute_C_metrics.py"
