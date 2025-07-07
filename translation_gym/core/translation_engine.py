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
        shutil.copytree(code_dir, self.output_dir/'source')

        code_dir = self.output_dir
        self.logger.log_status("Copied over the code to {}".format(code_dir.absolute()))
        self.source_manager = CSourceManager(code_dir/'source', logger=self.logger)
        
        # First compile the source code
        try:
            self.source_manager.compile(instrument=True, timeout=120) # Instrument first compile
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

        for unit in orchestrator.unit_iter(self.instrumentation_results):
            self.logger.log_status("Translating unit: {}".format(unit['name']))
            
            # Save a checkpoint to make sure we can reset if translation fails
            self.source_manager.save_state(unit)
            self.target_manager.save_state(unit)

            translation = translator.translate(unit)
            
            if translation is None:
                self.logger.log_failure("Translation failed")
                self.logger.log_output("No translation generated")
                self.logger.log_result({'unit': unit['name'],
                                        'type': unit['type'],
                                        'results': "No translation generated",
                                        'attempts': 0})
                continue
            
            result = validator.validate(unit, translation, self.source_manager, self.target_manager, self.test_manager)

            for i in range(self.num_attempts):
                self.logger.log_status(f"Attempt {i+1}/{self.num_attempts}")

                if result['success']:
                    self.logger.log_success("Translation succeeded")
                    orchestrator.update_state(unit, translation)
                    break
                else:
                    self.logger.log_failure("Translation failed")
                    self.logger.log_output(result['category'])
                    self.logger.log_output(result['message'])
                    self.source_manager.reset_unit(unit)
                    self.target_manager.reset_unit(unit)
                    if i == self.num_attempts - 1:
                        break
                    translation = translator.repair(result)

                    if translation is None:
                        self.logger.log_failure("Repair did not produce any code")
                        result = {
                            'success': False,
                            'category': 'repair_failed',
                            'message': 'translator.repair() returned None'
                        }
                        break
                    result = validator.validate(unit, translation, self.source_manager, self.target_manager, self.test_manager)
            
            self.logger.log_result({'unit': unit['name'],
                                    'type': unit['type'],
                                   'results': "Success" if result['success'] else result['category'],
                                   'attempts': i+1})
    
        orchestrator.prune(validator, self.test_manager)
        
    
    def print_results(self):
        # Draw an ascii table with the results
        import prettytable
        table = prettytable.PrettyTable()
        table.field_names = ["Unit", "Result", "Attempts"]
        results = self.logger.get_results()
        for result in results:
            table.add_row([result['unit'], result['results'], result['attempts']])
        table.add_divider()
        table.add_row(['Overall', "{}/{}".format(len([r for r in results if r['results'] == "Success"]), len(results)), ''])
        print(table)
        return
