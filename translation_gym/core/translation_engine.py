from translation_gym.helpers import *
from translation_gym.core.source_manager import SourceManager
from translation_gym.core.test_manager import TestManager
from translation_gym.modules.orchestrator import Orchestrator
from translation_gym.modules.translator import Translator
from translation_gym.modules.validator import Validator

class TranslationEngine:

    def __init__(self,
                dataset: dict,
                output_dir: str,
                model: str,
                num_attempts: int=5,
                verbose: bool=False):
        
        self.dataset = dataset
        self.verbose = verbose
        self.output_dir = Path(output_dir)
        self.setup() # Sets up self.source_manager
        self.num_attempts = num_attempts
        self.log_file = Path(self.output_dir, 'log.json')
        self.log = {'date': f"{datetime.datetime.now()}",
                    'attempts': num_attempts,
                    'model': model,
                    'results': []}

        with open(self.log_file, 'w') as f:
            f.write(json.dumps(self.log, indent=4))

    def setup(self):

        code_dir = Path("data")/Path(self.dataset["code_dir"])
        assert Path(code_dir).exists(), f"Code directory {code_dir} does not exist"

        prCyan("Translating code in directory: {}".format(code_dir.absolute()))

        # Creating new subdirectories
        output_dir = Path(self.output_dir)
        if output_dir.exists():
            prRed(f"Directory {output_dir} already exists. Please remove it before running the script.")
            raise FileExistsError(f"Directory {output_dir} already exists. Please remove it before running the script.")
            
        shutil.copytree('rust_wrapper', output_dir)
        shutil.copytree(code_dir, output_dir/'c_src')

        code_dir = output_dir
        prCyan("Copied over the code to {}".format(code_dir.absolute()))
        self.source_manager = SourceManager(code_dir)
        target = self.source_manager.get_bin_target()
        self.source_manager.set_cargo_bin_target(target)

        try:
            self.source_manager.compile(timeout=120) # Increase time for first compile
            prGreen("Compilation succeeded")
        except CompileException as e:
            prRed("Compilation failed")
            if self.verbose:
                prLightGray(e)
            raise CompileException(e)
        
        executable = self.source_manager.get_executable()
        prGreen("Generated executable: {}".format(executable))

        test_docker = self.dataset['test_docker_image']
        self.test_manager = TestManager(test_docker, verbose=self.verbose)
        test_res = self.test_manager.run_tests(self.source_manager)
        if test_res['status'] == "passed":
            prGreen("Tests passed")
            self.instrumentation_results = test_res['instrumentation']
        else:
            raise Exception(f"Tests failed: {test_res['error']}.")
        
    def run(self,
            orchestrator: Orchestrator,
            translator: Translator,
            validator: Validator):

        for func in orchestrator.function_iter(self.source_manager, self.instrumentation_results):
            prCyan("Translating function: {}".format(func['name']))
            translation = translator.translate(func, self.source_manager, self.verbose)
            result = validator.validate(func, translation, self.source_manager, self.test_manager)

            for i in range(self.num_attempts):
                prCyan(f"Attempt {i+1}/{self.num_attempts}")

                if result['success']:
                    prGreen("Translation succeeded")
                    break
                else:
                    prRed("Translation failed")
                    if self.verbose:
                        prLightGray(result['message'])
                    self.source_manager.reset_func(func)
                    if i == self.num_attempts - 1:
                        break
                    translation = translator.repair(result, self.source_manager, self.verbose)
                    result = validator.validate(func, translation, self.source_manager, self.test_manager)
            
            self.log['results'].append({'function': func['name'],
                                   'results': "Success" if result['success'] else result['category'],
                                   'attempts': i+1})
            with open(self.log_file, 'w') as f:
                f.write(json.dumps(self.log, indent=4))
    
    def print_results(self):
        # Draw an ascii table with the results
        import prettytable
        table = prettytable.PrettyTable()
        table.field_names = ["Function", "Result", "Attempts"]
        for result in self.log['results']:
            table.add_row([result['function'], result['results'], result['attempts']])
        table.add_divider()
        table.add_row(['Overall', "{}/{}".format(len([r for r in self.log['results'] if r['results'] == "Success"]), len(self.log['results'])), ''])
        print(table)
        return
