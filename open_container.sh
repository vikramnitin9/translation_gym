# Usage ./open_container.sh

DOCKER_SOCKET=$(docker context inspect | grep '"Host"' | head -n1 | sed -E 's/.*"Host": *"unix:\/\/([^"]+)".*/\1/')

# -u 0:0 is because of the rootless Docker setup.
# UID:GID on the host is mapped to 0:0 on the container
# See https://forums.docker.com/t/why-is-rootless-docker-still-running-as-root-inside-container/134985
docker run -it \
    -u 0:0 \
    -v $PWD/output:/app/output \
    -v $PWD/translation_gym:/app/translation_gym \
    -v $PWD/main.py:/app/main.py \
    -v $PWD/compute_metrics.py:/app/compute_metrics.py \
    -v $PWD/data:/app/data \
    -v $DOCKER_SOCKET:/var/run/docker.sock \
    translation_gym:latest \
    /bin/bash