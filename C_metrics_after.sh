#!/usr/bin/env bash
# -----------------------------------------------------------
# run_C_metrics.sh
# -----------------------------------------------------------
# Mounts your host project into /project so that
# compute_C_metrics.py can find data/… and invoke nested Docker
# without “mounts denied” errors.
# -----------------------------------------------------------
set -euo pipefail

# 1) make sure output/ exists (if your script writes there)
mkdir -p output

# 2) locate Docker socket
DOCKER_SOCKET=$(docker context inspect \
  | grep -m1 '"Host"' \
  | sed -E 's/.*"unix:\/\/([^"]+)".*/\1/')
[[ -S $DOCKER_SOCKET ]] || { echo "❌ Docker socket not found"; exit 1; }

# 3) run your Python driver inside the container,
#    with your host project mounted at /project
docker run --rm -it \
    -u 0:0 \
    -v "$PWD":/project \
    -v "$DOCKER_SOCKET":/var/run/docker.sock \
    translation_gym:latest \
    /bin/bash -lc "\
      cd /project && \
      python3 compute_C_metrics_after.py \
    "
