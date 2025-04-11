from translation_gym.helpers import *

class Validator:

    def __init__(self, compile_attempts=5):
        self.compile_attempts = 5

    def run_tests(self, source_manager):
        executable = source_manager.get_executable()
        test_paths = source_manager.get_test_paths()
        # Run each test file
        for test_path in test_paths:
            # Run the test file
            cmd = f'PATH="{executable.parent.absolute()}:$PATH" bash {test_path}'
            try:
                run(cmd)
                prGreen(f"Test passed: {test_path}")
            except RunException as e:
                prRed(f"Test failed: {test_path}")
                # If the test fails, return False
                return (False, str(e))
        # If all tests pass, return True
        return (True, '')

    def validate(self, func, translation, source_manager):
        source_manager.insert_translation(func, translation)

        compile_success = False
        error_message = ''
        # Try 5 times to compile, in case there is a timeout or mysterious linker error
        for _ in range(5):
            try:
                source_manager.compile()
                compile_success = True
                break
            except CompileException as e:
                error_message = str(e)
                if "Timeout" in str(e):
                    prRed("Timeout. Trying again.")
                    continue
                elif "rust-lld: error:" in str(e):
                    prRed("Linker error. Cleaning up and trying again.")
                    source_manager.cleanup()
                    continue

        if not compile_success:
            return {"success": False,
                    "category": "Compile Error",
                    "message" : error_message}
        
        # If we get here, the code compiled successfully
        # Run the test suite
        test_success, error_message = self.run_tests(source_manager)
        if not test_success:
            return {"success": False,
                    "category": "Test Failure",
                    "message" : error_message}

        return {"success": True,
                "category": "",
                "message" : ""}