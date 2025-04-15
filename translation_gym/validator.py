from translation_gym.helpers import *

class Validator:

    def __init__(self, compile_attempts=5):
        self.compile_attempts = 5

    def validate(self, func, translation, source_manager, test_manager):
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
        executable = source_manager.get_executable()
        test_res = test_manager.run_tests(executable, stop_on_failure=True)
        if test_manager.passed():
            return {"success": True,
                "category": "",
                "message" : ""}
        else:
            failed_test = [res for res in test_res if res['status'] == 'failed'][0]
            return {"success": False,
                    "category": "Test Failure",
                    "message" : failed_test['error']}