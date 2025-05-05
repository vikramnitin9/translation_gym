from translation_gym.helpers import *
from translation_gym.core.source_manager import CSourceManager
from translation_gym.core.target_manager import RustTargetManager
from translation_gym.core.test_manager import TestManager
from translation_gym.modules.orchestrator import Orchestrator
from translation_gym.modules.translator import Translator
from translation_gym.modules.validator import Validator

class TranslationEngine:

    def __init__(self,
                dataset: dict,
                output_dir: str,
                num_attempts: int,
                logger: Logger):
        
        self.dataset = dataset
        self.logger = logger
        self.output_dir = Path(output_dir)
        self.setup() # Sets up source_manager and target_manager
        self.num_attempts = num_attempts
        self.translated = {}

    def get_source_manager(self):
        return self.source_manager

    def get_target_manager(self):
        return self.target_manager

    def get_test_manager(self):
        return self.test_manager

    def setup(self):

        code_dir = Path("data")/Path(self.dataset["code_dir"])
        assert Path(code_dir).exists(), f"Code directory {code_dir} does not exist"

        self.logger.log_status("Translating code in directory: {}".format(code_dir.absolute()))

        # Creating new subdirectories
        self.output_dir.mkdir(parents=True, exist_ok=True)
        shutil.copytree(code_dir, self.output_dir/'c_src')

        code_dir = self.output_dir
        self.logger.log_status("Copied over the code to {}".format(code_dir.absolute()))
        self.source_manager = CSourceManager(code_dir/'c_src', logger=self.logger)
        
        # First compile the source code
        try:
            self.source_manager.compile(timeout=120) # Increase time for first compile
            self.logger.log_success("Compilation succeeded")
        except CompileException as e:
            self.logger.log_failure("Compilation failed")
            self.logger.log_output(e)
            raise CompileException(e)

        src_build_path = self.source_manager.get_build_path()

        self.target_manager = RustTargetManager(code_dir, src_build_path, logger=self.logger)
        target = self.source_manager.get_bin_target()
        self.target_manager.setup()
        self.target_manager.set_target_name(target)

        # Then compile the target language wrapper and link it with the compiled source
        try:
            self.target_manager.compile(timeout=120) # Increase time for first compile
            self.logger.log_success("Compilation succeeded")
        except CompileException as e:
            self.logger.log_failure("Compilation failed")
            self.logger.log_output(e)
            raise CompileException(e)
        
        executable = self.target_manager.get_executable()
        self.logger.log_success("Generated executable: {}".format(executable))

        test_docker = self.dataset['test_docker_image']
        self.test_manager = TestManager(test_docker, logger=self.logger)
        test_res = self.test_manager.run_tests(self.target_manager)
        if test_res['status'] == "passed":
            self.logger.log_success("Tests passed")
            self.instrumentation_results = test_res['instrumentation']
        else:
            raise Exception(f"Tests failed: {test_res['error']}.")
        
    def run(self,
            orchestrator: Orchestrator,
            translator: Translator,
            validator: Validator):

        for func in orchestrator.function_iter(self.source_manager, self.instrumentation_results):
            self.logger.log_status("Translating function: {}".format(func['name']))

            # This is the part where we collect info about the called functions.
            # We first need to compile the source to get the most up-to-date static library
            self.source_manager.compile()
            rust_static_analysis = self.target_manager.get_static_analysis_results()
            for i, called_func in enumerate(func['calledFunctions']):
                translated_rust_fns = [f for f in rust_static_analysis['functions'] if f['name'] == (called_func['name'] + "_rust")]
                if len(translated_rust_fns) != 0:
                    assert len(translated_rust_fns) == 1
                    func['calledFunctions'][i]['signature'] = translated_rust_fns[0]['signature']
                    func['calledFunctions'][i]['translated'] = True
                else:
                    matching_rust_fn = [f for f in rust_static_analysis['functions'] if f['name'] == called_func['name']]
                    if len(matching_rust_fn) == 0:
                        # This can happen if bindgen fails to find this function for some reason.
                        self.logger.log_failure(f"Function {called_func['name']} not found in bindgen headers")
                        func['calledFunctions'][i]['signature'] = None
                        func['calledFunctions'][i]['translated'] = False
                        continue
                    assert len(matching_rust_fn) == 1
                    func['calledFunctions'][i]['signature'] = matching_rust_fn[0]['signature']
                    func['calledFunctions'][i]['translated'] = False
                    
            translation = translator.translate(func, self.source_manager)
            result = validator.validate(func, translation, self.source_manager, self.target_manager, self.test_manager)

            for i in range(self.num_attempts):
                self.logger.log_status(f"Attempt {i+1}/{self.num_attempts}")

                if result['success']:
                    self.logger.log_success("Translation succeeded")
                    break
                else:
                    self.logger.log_failure("Translation failed")
                    self.logger.log_output(result['category'])
                    self.logger.log_output(result['message'])
                    self.source_manager.reset_func(func)
                    self.target_manager.reset_func(func)
                    if i == self.num_attempts - 1:
                        break
                    translation = translator.repair(result, self.source_manager)
                    result = validator.validate(func, translation, self.source_manager, self.target_manager, self.test_manager)
            
            if result['success']:
                self.logger.log_success("Translation succeeded")
                self.translated[func['name']] = translation['func']
            
            self.logger.log_result({'function': func['name'],
                                   'results': "Success" if result['success'] else result['category'],
                                   'attempts': i+1})
    
    def print_results(self):
        # Draw an ascii table with the results
        import prettytable
        table = prettytable.PrettyTable()
        table.field_names = ["Function", "Result", "Attempts"]
        results = self.logger.get_results()
        for result in results:
            table.add_row([result['function'], result['results'], result['attempts']])
        table.add_divider()
        table.add_row(['Overall', "{}/{}".format(len([r for r in results if r['results'] == "Success"]), len(results)), ''])
        print(table)
        return
