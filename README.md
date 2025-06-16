# Translation Gym

https://github.com/user-attachments/assets/d421db0d-9038-4213-bd80-d0a641965e63

This is an environment for quickly implementing and benchmarking LLM-based program translation agents. As of now, we support only C to Rust translation.

## Why use this tool?

Implementing an LLM-based solution for full-project translation can be tricky. You can't translate the entire project at one go, so you need to break it up into individual functions. These functions have to be extracted with their dependencies and translated. Then you need to incorporate this function into the project, and test to see that it still works.

Translation Gym takes care of all this effort! It gives you:
- [x] a function body
- [x] the functions that it calls
- [x] values of each argument and the returned value, from a sample execution 
- [x] any global variables used by this function
- [x] definitions of any structure or enum used in this function

In exchange, you need to provide it with:
- [x] the function translation
- [x] "glue code" in the form of a wrapper function to interface with the source language

Translation Gym will merge this into the project, compile it, run tests, and give you:
- [x] compiler feedback
- [x] stdout/stderr of the run
- [ ] a runtime trace with values of each argument and the returned value

You can use this feedback to repair your translation and provide Translation Gym with:
- [x] the repaired function translation
- [x] repaired "glue code"

In this manner, you can translate all the functions in the project! The entire process is highly customizable, and you can implement your own `Translator`, `Validator` and `Orchestrator` agents (or use our default implementations). Additionally, we also provide a suite of C projects from Coreutils out-of-the-box for benchmarking, and you can add your own datasets too.

## Quickstart

The easiest way to run this tool is with Docker and docker-compose. If you do not already have Docker installed, follow the instructions [here for Docker](https://docs.docker.com/engine/install/), and [here for docker-compose](https://docs.docker.com/compose/install/). To build the Docker container for our tool, run the following script:
```sh
bash build.sh
```
Each test dataset is built into a separate Docker container. Run the following commands to build all datasets (this should take about 10 minutes in total):
```sh
cd data
export USER_ID=$(id -u)
export GROUP_ID=$(id -g)
docker-compose build coreutils && docker-compose build
```
For LLM-based translation, we support OpenAI, Anthropic, and Google models. First put your API key in `models/.env`. For example:
```sh
# Use `ANTHROPIC_API_KEY`, `GOOGLE_API_KEY` or `OPENAI_API_KEY`, depending on your use case
echo 'OPENAI_API_KEY="<your_key_here>"' > translation_gym/models/.env
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
