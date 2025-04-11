from translation_gym.helpers import *

class TranslationEngine:

    def __init__(self,
                code_dir: Path,
                test_dir: Path,
                model: str,
                num_attempts: int=5,
                verbose: bool=False):
        
        self.code_dir = code_dir
        self.test_dir = test_dir
        self.verbose = verbose
        self.setup() # Sets up self.source_manager
        self.num_attempts = num_attempts
        self.log_file = Path(self.code_dir, 'log.json')

        self.log = {'date': f"{datetime.datetime.now()}",
                    'attempts': num_attempts,
                    'model': model,
                    'results': []}

        with open(self.log_file, 'w') as f:
            f.write(json.dumps(self.log, indent=4))

    def setup(self):

        prCyan("Translating code in directory: {}".format(self.code_dir.absolute()))

        assert Path(self.code_dir, 'compile_commands.json').exists(),\
                f"{self.code_dir}/compile_commands.json does not exist"
        
        # Creating new subdirectories
        # Create a folder called `rust_bench` in the parent directory of the code dir
        rust_bench_dir = Path(self.code_dir).parent / 'rust_bench'
        if rust_bench_dir.exists():
            # Call it rust_bench{i} where i is the lowest number that doesn't exist
            i = 2
            while (rust_bench_dir.parent / f'rust_bench{i}').exists():
                i += 1
            rust_bench_dir = (rust_bench_dir.parent / f'rust_bench{i}')
            
        shutil.copytree('rust_wrapper', rust_bench_dir)
        shutil.copytree(self.code_dir, rust_bench_dir/'c_src')

        self.code_dir = rust_bench_dir
        prCyan("Copied over the code to {}".format(self.code_dir.absolute()))
        self.source_manager = SourceManager(self.code_dir, self.test_dir)
        target = self.source_manager.get_bin_target()
        self.source_manager.set_cargo_bin_target(target)

        try:
            self.source_manager.compile()
            prGreen("Compilation succeeded")
        except CompileException as e:
            prRed("Compilation failed")
            if self.verbose:
                prLightGray(e)
            raise CompileException(e)
        
        executable = self.source_manager.get_executable()
        prGreen("Generated executable: {}".format(executable))
        test_paths = self.source_manager.get_test_paths()
        selected_tests = []
        prCyan("Running tests in directory: {}".format(self.test_dir))
        # Run each test file
        for test_path in test_paths:
            # Run the test file
            cmd = f'PATH="{executable.parent.absolute()}:$PATH" bash {test_path}'
            try:
                run(cmd)
                prGreen(f"Test passed: {test_path}")
                selected_tests.append(test_path)
            except RunException as e:
                prRed(f"Test failed: {test_path}. This will be skipped")
                if self.verbose:
                    prCyan(f"Command: {cmd}")
                    prLightGray(e)
        
        self.source_manager.set_test_paths(selected_tests)
        prCyan("Tests selected: {}".format([t.name for t in selected_tests]))

        return 
    
    def run(self,
            orchestrator: Orchestrator,
            translator: Translator,
            validator: Validator):

        for func in orchestrator.function_iter(self.source_manager):
            prCyan("Translating function: {}".format(func['name']))
            translation = translator.translate(func, self.source_manager, self.verbose)
            result = validator.validate(func, translation, self.source_manager)

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
                    result = validator.validate(func, translation, self.source_manager)
            
            self.log['results'].append({'function': func['name'],
                                   'results': "Success" if result['success'] else result['category']})
            with open(self.log_file, 'w') as f:
                f.write(json.dumps(self.log, indent=4))