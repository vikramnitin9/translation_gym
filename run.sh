# Usage ./run.sh <dataset_name> <model_name>
# <dataset_name> must be in data/datasets.json and <model_name> must be in translation_gym/models/__init__.py
mkdir -p output

DOCKER_SOCKET=$(docker context inspect | grep '"Host"' | head -n1 | sed -E 's/.*"Host": *"unix:\/\/([^"]+)".*/\1/')

# Generate a random string of length 10 for the output directory
suffix=$(openssl rand -base64 32 | tr -dc '[:alnum:]' | head -c 10)

# -u 0:0 is because of the rootless Docker setup.
# UID:GID on the host is mapped to 0:0 on the container
# See https://forums.docker.com/t/why-is-rootless-docker-still-running-as-root-inside-container/134985
docker run -it \
    -u 0:0 \
    -v $PWD/output:/app/output \
    -v $PWD/translation_gym:/app/translation_gym \
    -v $PWD/main.py:/app/main.py \
    -v $PWD/data:/app/data \
    -v $DOCKER_SOCKET:/var/run/docker.sock \
    translation_gym:latest \
    /bin/bash -c "python main.py --dataset $1 --model $2 --output_dir /app/output/$1_$suffix --verbose"
