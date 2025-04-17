# Translation Gym

This is an environment for quickly implementing and benchmarking LLM-based program translation agents. As of now, we support only C to Rust translation.

## Quickstart

First put your OpenAI API key in `models/.env`.
```sh
echo 'OPENAI_API_KEY="<your_key_here>"' > translation_gym/models/.env
```
The easiest way to run this tool is with Docker. If you do not already have Docker installed, follow the instructions [here](https://docs.docker.com/engine/install/). This script builds the Docker container for the tool.
```sh
bash build.sh
```
Each test dataset is built into a separate Docker container. If you have `docker-compose`, then you can simply run
```sh
cd data
docker-compose build
```
If you don't have `docker-compose`, then you have to manually build each container like this:
```sh
docker build -f toy/tests/Dockerfile -t toy:latest .
docker build -f coreutils/tests/cat/Dockerfile -t cat:latest .
...
```
Now you are ready to run translation.
```sh
bash run.sh toy gpt4o
```
Here, `toy` is the name of a sample C program, corresponding to a dataset config in [`data/datasets.json`](data/datasets.json). `gpt4o` is a model name, defined in [`translation_gym/models/__init__.py`](translation_gym/models/__init__.py).
You should see output like this:
```
Translating code in directory: /app/data/toy
Copied over the code to /app/output/toy
Found executable target: toy
Compilation succeeded
Generated executable: /app/output/toy/target/debug/toy
Running tests against the following executable: /app/output/toy/target/debug/toy
Test passed: data/toy/tests/test.sh
Translating function: subtract
Calling LLM for translation
LLM response received
Running tests against the following executable: /app/output/toy/target/debug/toy
Attempt 1/5
Translation succeeded
```
At the end of this process, the translated Rust project will be in `output/toy`. The results will be printed like this:
```
+-------------+---------------+----------+
|   Function  |     Result    | Attempts |
+-------------+---------------+----------+
|   subtract  |    Success    |    1     |
| concatenate |    Success    |    1     |
|     add     |    Success    |    1     |
|    main_0   |    Success    |    5     |
+-------------+---------------+----------+
|   Overall   |      4/4      |          |
+-------------+---------------+----------+
```
These results are also logged to `output/toy/log.json`.

## Customization

`translation_gym` is designed to be extensively customizable! To that end, we are continually adding functionality to provide allow developers to implement a wider range of translation algorithms. If you have any specific requests, please open an issue.

Currently, `translation_gym` allows you to implement your own [`Orchestrator`](translation_gym/modules/orchestrator.py), [`Translator`](translation_gym/modules/translator.py), and [`Validator`](translation_gym/modules/validator.py). The templates for each of those classes are in [`translation_gym/modules`](translation_gym/modules). For example, to implement your own custom translation logic, you would inherit from `Translator` as follows:
```python
class MyTranslator(Translator):

    def __init__(self, arg1, ...):
        # Some code here
```
You need to implement two complusory methods, `translate` and `repair`.
```python
    def translate(self, func, source_manager, verbose=False):
        # Your translation logic here
        # Return a function translation in the specific format
    
    def repair(self, result, source_manager, verbose=False):
        # Your repair logic here
        # Return a repaired version of the function translation
```
Now you can use `MyTranslator` as part of the translation routine. Modify [`main.py`](main.py) as follows:
```python
...
orchestrator = DefaultOrchestrator()
my_translator = MyTranslator(arg1, ...) # Custom translation logic
validator = DefaultValidator(compile_attempts=5)

engine = TranslationEngine(dataset=dataset,
                            output_dir=args.output_dir,
                            model=args.model,
                            num_attempts=args.num_attempts,
                            verbose=args.verbose)

engine.run(translator=my_translator,
            orchestrator=orchestrator,
            validator=validator)
...
```
The translation can be run as usual:
```sh
bash run.sh toy gpt4o
```
