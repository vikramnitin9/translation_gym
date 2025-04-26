from translation_gym.helpers import *
from translation_gym.core.source_manager import CManager
from translation_gym.core.target_manager import RustManager
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
        self.setup() # Sets up source_manager and target_manager
        self.num_attempts = num_attempts
        self.log_file = Path(self.output_dir, 'log.json')
        self.log = {'date': f"{datetime.datetime.now()}",
                    'attempts': num_attempts,
                    'model': model,
                    'results': []}
        self.translated = {}

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
        
        output_dir.mkdir(parents=True, exist_ok=True)
        shutil.copytree(code_dir, output_dir/'c_src')

        code_dir = output_dir
        prCyan("Copied over the code to {}".format(code_dir.absolute()))
        self.source_manager = CManager(code_dir/'c_src')
        
        # First compile the source code
        try:
            self.source_manager.compile(timeout=120) # Increase time for first compile
            prGreen("Compilation succeeded")
        except CompileException as e:
            prRed("Compilation failed")
            if self.verbose:
                prLightGray(e)
            raise CompileException(e)

        src_build_path = self.source_manager.get_build_path()

        self.target_manager = RustManager(code_dir, src_build_path)
        target = self.source_manager.get_bin_target()
        self.target_manager.setup()
        self.target_manager.set_target_name(target)

        # Then compile the target language wrapper and link it with the compiled source
        try:
            self.target_manager.compile(timeout=120) # Increase time for first compile
            prGreen("Compilation succeeded")
        except CompileException as e:
            prRed("Compilation failed")
            if self.verbose:
                prLightGray(e)
            raise CompileException(e)
        
        executable = self.target_manager.get_executable()
        prGreen("Generated executable: {}".format(executable))

        test_docker = self.dataset['test_docker_image']
        self.test_manager = TestManager(test_docker, verbose=self.verbose)
        test_res = self.test_manager.run_tests(self.target_manager)
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

            # This is the part where we collect info about the called functions.
            rust_static_analysis = self.target_manager.get_static_analysis_results()
            for i, called_func in enumerate(func['calledFunctions']):
                translated_rust_fns = [f for f in rust_static_analysis if f['name'] == (called_func['name'] + "_rust")]
                if len(translated_rust_fns) != 0:
                    assert len(translated_rust_fns) == 1
                    func['calledFunctions'][i]['signature'] = translated_rust_fns[0]['signature']
                    func['calledFunctions'][i]['translated'] = True
                else:
                    matching_rust_fn = [f for f in rust_static_analysis if f['name'] == called_func['name']]
                    assert len(matching_rust_fn) == 1
                    func['calledFunctions'][i]['signature'] = matching_rust_fn[0]['signature']
                    func['calledFunctions'][i]['translated'] = False
                    
            translation = translator.translate(func, self.source_manager, self.verbose)
            result = validator.validate(func, translation, self.source_manager, self.target_manager, self.test_manager)

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
                    self.target_manager.reset_func(func)
                    if i == self.num_attempts - 1:
                        break
                    translation = translator.repair(result, self.source_manager, self.verbose)
                    result = validator.validate(func, translation, self.source_manager, self.target_manager, self.test_manager)
            
            if result['success']:
                prGreen("Translation succeeded")
                self.translated[func['name']] = translation['func']
            
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
