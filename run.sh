# Usage ./run.sh <dataset_name> <model_name>
# <dataset_name> must be in data/datasets.json and <model_name> must be in translation_gym/models/__init__.py
mkdir output
rm -rf output/$1
docker run -it -v $PWD/output:/app/output translation_gym:latest python -m translation_gym.main --dataset $1 --model $2 --output_dir /app/output/$1