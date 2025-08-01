from translation_gym.helpers import *

class Validator:
    """
    Base class for validators. This class is responsible for validating the translated code.
    """
    def validate(self, unit, translation, source_manager, target_manager, test_manager):
        """
        Validate the translated code.

        :param unit: The original unit of code
        :param translation: The translated unit
        :param source_manager: The source manager
        :param target_manager: The target manager
        :param test_manager: The test manager
        :return: A dictionary with the validation result
                {"success": True/False,
                 "category": "Compile Error" or "Test Failure",
                 "message": "Error message"}
        """
        raise NotImplementedError("Subclasses must implement this method")
    
    def compile_and_test(self, source_manager, target_manager, test_manager):
        """
        Compile the current code and run tests.

        :param source_manager: The source manager
        :param target_manager: The target manager
        :param test_manager: The test manager
        :return: A dictionary with the compilation and test results
                {"success": True/False,
                 "category": "Compile Error" or "Test Failure",
                 "message": "Error message"}
        """
        raise NotImplementedError("Subclasses must implement this method")
    
    
class DefaultValidator(Validator):

    def __init__(self, compile_attempts, logger):
        self.compile_attempts = compile_attempts
        self.logger = logger

    def validate(self, unit, translation, source_manager, target_manager, test_manager):
        if unit['type'] == 'functions':
            source_manager.remove_func(unit)
            target_manager.insert_translation(unit, translation['func'])
            target_manager.insert_translation(unit, translation['wrapper'])
            target_manager.insert_imports(unit, translation['imports'])
        elif unit['type'] == 'structs' or unit['type'] == 'globals':
            target_manager.insert_translation(unit, translation['struct'])
            target_manager.insert_imports(unit, translation['imports'])
        elif unit['type'] == 'enums':
            target_manager.insert_translation(unit, translation['enum'])
            target_manager.insert_imports(unit, translation['imports'])
        else:
            raise NotImplementedError(f"Translation not implemented for {unit['type']}")
        
        return self.compile_and_test(source_manager, target_manager, test_manager)

    def compile_and_test(self, source_manager, target_manager, test_manager):

        compile_success = False
        error_message = ''
        # Try 5 times to compile, in case there is a timeout or mysterious linker error
        for _ in range(self.compile_attempts):
            try:
                source_manager.cleanup()
                source_manager.compile()
                compile_success = True
                break
            except CompileException as e:
                error_message = str(e)
                if "Timeout" in str(e):
                    self.logger.log_failure("Timeout. Trying again.")
                    continue
                break

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
                break # Unless it's a timeout or linker error, we don't want to retry

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