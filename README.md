# Translation Gym

This is an environment for quickly implementing and benchmarking LLM-based program translation agents. As of now, we support only C to Rust translation.

First put your OpenAI API key in `models/.env`.
```sh
echo 'OPENAI_API_KEY="<your_key_here>"' > translation_gym/models/.env
```
The easiest way to run this tool is with Docker. This script builds the Docker container.
```sh
bash build.sh
```
Now you are ready to run translation.
```sh
bash run.sh toy gpt4o
```
Here, `toy` is the name of a sample program, corresponding to a dataset config in `data/datasets.json`. `gpt4o` is a model name, defined in `translation_gym/models/__init__.py`.