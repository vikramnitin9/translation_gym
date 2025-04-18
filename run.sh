# Usage ./run.sh <dataset_name> <model_name>
# <dataset_name> must be in data/datasets.json and <model_name> must be in translation_gym/models/__init__.py
mkdir -p output

DOCKER_SOCKET=$(docker context inspect | jq -r .[0].Endpoints.docker.Host | sed "s^unix://^^")

# Generate a random string of length 10 for the output directory
suffix=$(openssl rand -base64 32 | tr -dc '[:alnum:]' | head -c 10)

docker run -it \
    -u 0:0 \
    -v $PWD/output:/app/output \
    -v $PWD/translation_gym:/app/translation_gym \
    -v $PWD/main.py:/app/main.py \
    -v $PWD/data:/app/data \
    -v $DOCKER_SOCKET:/var/run/docker.sock \
    translation_gym:latest \
    /bin/bash -c "python main.py --dataset $1 --model $2 --output_dir /app/output/$1_$suffix --verbose"