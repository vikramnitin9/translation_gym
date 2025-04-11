# Translation Gym

This is an environment for quickly implementing and benchmarking LLM-based program translation agents. As of now, we support only C to Rust translation.

First setup our C parser utility:
```sh
cd parsec
mkdir build && cd build
cmake .. && make -j
export PARSEC_BUILD_DIR="$(pwd)"
cd ../..
```

Now you are ready to run translation.
```sh
python -m translation_gym.main --code_dir data/toy --test_dir data/toy/tests --verbose --model gpt4o
```