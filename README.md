# Translation Gym

This is an environment for quickly implementing and benchmarking LLM-based program translation agents. As of now, we support only C to Rust translation.

## Quickstart

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
Here, `toy` is the name of a sample C program, corresponding to a dataset config in `data/datasets.json`. `gpt4o` is a model name, defined in `translation_gym/models/__init__.py`.
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
At the end of this process, the translated Rust project will be in `output/toy`.

## Customization

`translation_gym` is designed to be extensively customizable! To that end, we are continually adding functionality to provide allow developers to implement a wider range of translation algorithms. If you have any specific requests, please open an issue.

Currently, `translation_gym` allows you to implement your own `Orchestrator`, `Translator`, and `Validator`. The templates for each of those classes are in `translation_gym/modules`. For example, to implement your own custom translation logic, you would inherit from `Translator` as follows:
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
Now you can use `MyTranslator` as part of the translation routine. Here is the relevant portion of `main.py`, modified:
```python
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
```
