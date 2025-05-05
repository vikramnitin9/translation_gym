from translation_gym.helpers import *

class Validator:
    """
    Base class for validators. This class is responsible for validating the translated code.
    """
    def validate(self, func, translation, source_manager, target_manager, test_manager):
        """
        Validate the translated code.

        :param func: The original function
        :param translation: The translated function
        :param source_manager: The source manager
        :param target_manager: The target manager
        :param test_manager: The test manager
        :return: A dictionary with the validation result
                {"success": True/False,
                 "category": "Compile Error" or "Test Failure",
                 "message": "Error message"}
        """
        raise NotImplementedError("Subclasses must implement this method")
    
    
class DefaultValidator(Validator):

    def __init__(self, compile_attempts, logger):
        self.compile_attempts = compile_attempts
        self.logger = logger

    def validate(self, func, translation, source_manager, target_manager, test_manager):
        source_manager.remove_func(func)
        target_manager.insert_translation(func, translation)

        compile_success = False
        error_message = ''
        # Try 5 times to compile, in case there is a timeout or mysterious linker error
        for _ in range(self.compile_attempts):
            try:
                source_manager.compile()
                compile_success = True
                break
            except CompileException as e:
                error_message = str(e)
                if "Timeout" in str(e):
                    self.logger.log_failure("Timeout. Trying again.")
                    continue
                elif "rust-lld: error:" in str(e):
                    self.logger.log_failure("Linker error. Cleaning up and trying again.")
                    source_manager.cleanup()
                    continue

        if not compile_success:
            return {"success": False,
                    "category": "Compile Error",
                    "message" : error_message}
        
        compile_success = False
        error_message = ''    
        for _ in range(self.compile_attempts):
            try:
                target_manager.compile()
                compile_success = True
                break
            except CompileException as e:
                error_message = str(e)
                if "Timeout" in str(e):
                    self.logger.log_failure("Timeout. Trying again.")
                    continue
                elif "rust-lld: error:" in str(e):
                    self.logger.log_failure("Linker error. Cleaning up and trying again.")
                    target_manager.cleanup()
                    continue

        if not compile_success:
            return {"success": False,
                    "category": "Compile Error",
                    "message" : error_message}
        
        # If we get here, the code compiled successfully
        # Run the test suite
        test_res = test_manager.run_tests(target_manager)
        if test_res['status'] == 'passed':
            return {"success": True,
                "category": "",
                "message" : ""}
        else:
            return {"success": False,
                    "category": "Test Failure",
                    "message" : test_res['error']}