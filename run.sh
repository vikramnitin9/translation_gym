# Usage ./run.sh <dataset_name> <model_name>
# <dataset_name> must be in data/datasets.json and <model_name> must be in translation_gym/models/__init__.py
mkdir -p output
rm -rf output/$1

docker run -it \
    -u $USER_ID:$GROUP_ID \
    -v $PWD/output:/app/output \
    -v $PWD/translation_gym:/app/translation_gym \
    -v $PWD/data:/app/data \
    translation_gym:latest \
    /bin/bash -c "python -m translation_gym.main --dataset $1 --model $2 --output_dir /app/output/$1"