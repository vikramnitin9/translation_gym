from translation_gym.models import get_model_from_name, ModelException
from translation_gym.helpers import *
from translation_gym.prompts import *

class Translator:

    """
    Base class for translators. This class is responsible for translating the code.
    """

    def translate(self, func, source_manager, target_manager):
        """
        Translate the given function to Rust.
        :param func: The function to translate
            {'name': 'function name',
             'body': 'function body',
             'signature': 'function signature',
             'functions': [
                {
                    'binding': '<func1, FFI binding signature>',
                    'translated': '<func1, translated version signature>
                },
                ...
             ],
             'structs': [
                {
                    'binding': '<struct1, FFI binding definition>',
                    'translated': '<struct1, translated version definition>'
                },
                ...
             ],
             'globals': [
                {
                    'binding': '<global1, FFI binding declaration>',
                },
                ...
             ],
            }
        :param source_manager: The source manager
        :return: A dictionary with the translated function
                {"func": "translated function",
                 "wrapper": "wrapper function",
                 "imports": "imports"}
        """
        raise NotImplementedError("Subclasses must implement this method")
    
    def repair(self, result, source_manager, target_manager):
        """
        Repair the given function.
        :param result: The result of the translation
            {"success": True/False,
             "category": "Compile Error" or "Test Failure",
             "message": "Error message"
            }
        :param source_manager: The source manager
        :return: A dictionary with the repaired function
                {"func": "repaired function body",
                 "wrapper": "repaired wrapper function",
                 "imports": "imports"}
        """
        raise NotImplementedError("Subclasses must implement this method")


class DefaultTranslator(Translator):

    def __init__(self, model, logger):
        self.model = get_model_from_name(model)
        self.logger = logger
        self.conversation = []
        self.unit = None
    
    def translate(self, unit, source_manager, target_manager):

        if unit['type'] == 'functions':
            translation_prompt = construct_prompt_for_func(unit)
        elif unit['type'] == 'structs':
            translation_prompt = construct_prompt_for_struct(unit)
        elif unit['type'] == 'globals':
            translation_prompt = construct_prompt_for_global(unit)
        else:
            raise NotImplementedError("Translation not implemented for this unit type")

        self.conversation = [{'role': 'system', 'content': 'You are an intelligent code assistant'},
                            {'role': 'user', 'content': translation_prompt.strip()}]
        self.unit = unit

        self.logger.log_output(translation_prompt)

        for _ in range(5):
            try:
                self.logger.log_status("Calling LLM for translation")
                response = self.model.gen(self.conversation, top_k=1, temperature=0)[0]
                self.conversation.append({'role': 'assistant', 'content': response})
                self.logger.log_success("LLM response received")
                self.logger.log_output(response)
                # Parse the response and extract the text between either
                # <FUNC>...</FUNC>, <IMPORT>...</IMPORT> or <WRAPPER>...</WRAPPER> tags
                if '<IMPORTS>\n' in response:
                    imports = response.split('<IMPORTS>\n')[1].split('</IMPORTS>')[0]
                else:
                    imports = ''
                imports = imports.replace('```rust', '').replace('```', '').strip()

                if unit['type'] == 'functions':
                    if '<FUNC>\n' not in response:
                        self.logger.log_failure("Response does not contain <FUNC> tag. Trying again.")
                        continue
                    if '<WRAPPER>\n' not in response:
                        self.logger.log_failure("Response does not contain <WRAPPER> tag. Trying again.")
                        continue
                    function_trans = response.split('<FUNC>\n')[1].split('</FUNC>')[0]
                    wrapper = response.split('<WRAPPER>\n')[1].split('</WRAPPER>')[0]

                    # Remove any ```rust and ``` tags from function_trans and wrapper
                    function_trans = function_trans.replace('```rust', '').replace('```', '').strip()
                    wrapper = wrapper.replace('```rust', '').replace('```', '').strip()
                    return {
                        'func': function_trans,
                        'wrapper': wrapper,
                        'imports': imports,
                    }
                
                elif unit['type'] == 'structs' or unit['type'] == 'globals':
                    if '<STRUCT>\n' not in response:
                        self.logger.log_failure("Response does not contain <STRUCT> tag. Trying again.")
                        continue
                    struct_trans = response.split('<STRUCT>\n')[1].split('</STRUCT>')[0]
                    # Remove any ```rust and ``` tags
                    struct_trans = struct_trans.replace('```rust', '').replace('```', '').strip()
                    return {
                        'struct': struct_trans,
                        'imports': imports,
                    }

                break
            except ModelException as e:
                self.logger.log_status(f"Model exception\n{e}\nTrying again")
                continue

        return None
    
    def repair(self, result, source_manager, target_manager):

        assert len(self.conversation) > 0, "Repair called before translation"

        if result['category'] == "Compile Error":
            prompt = ("The translation generated the following compile error:\n"
                    f"{result['message']}\n"
                    f"Please re-generate the translation.\n"
                    f"Remember to follow the same format with the appropriate tags <...> </...>.\n")
        elif result['category'] == "Test Failure":
            prompt = ("The translation failed tests. This was the command output:\n"
                    f"{result['message']}\n"
                    f"Please re-generate the translation.\n"
                    f"Remember to follow the same format with the appropriate tags <...> </...>.")
        else:
            raise NotImplementedError("Repair not implemented for this error type")
        
        self.conversation += [{'role': 'user', 'content': prompt.strip()}]

        while True:
            try:
                self.logger.log_status("Calling LLM for repair")
                response = self.model.gen(self.conversation, top_k=1, temperature=0)[0]
                self.conversation.append({'role': 'assistant', 'content': response})
                self.logger.log_success("LLM response received")
                self.logger.log_output(response)
                # Parse the response and extract the text between either
                # <FUNC>...</FUNC>, <IMPORT>...</IMPORT> or <WRAPPER>...</WRAPPER> tags
                if '<IMPORTS>\n' in response:
                    imports = response.split('<IMPORTS>\n')[1].split('</IMPORTS>')[0]
                else:
                    imports = ''
                imports = imports.replace('```rust', '').replace('```', '').strip()
                
                if self.unit['type'] == 'functions':
                    if '<FUNC>\n' not in response:
                        self.logger.log_failure("Response does not contain <FUNC> tag. Trying again.")
                        continue
                    if '<WRAPPER>\n' not in response:
                        self.logger.log_failure("Response does not contain <WRAPPER> tag. Trying again.")
                        continue
                    function_trans = response.split('<FUNC>\n')[1].split('</FUNC>')[0]
                    wrapper = response.split('<WRAPPER>\n')[1].split('</WRAPPER>')[0]
                    # Remove any ```rust and ``` tags from imports, function_trans and wrapper
                    function_trans = function_trans.replace('```rust', '').replace('```', '').strip()
                    wrapper = wrapper.replace('```rust', '').replace('```', '').strip()
                    return {
                        'func': function_trans,
                        'wrapper': wrapper,
                        'imports': imports,
                    }
                elif self.unit['type'] == 'structs' or self.unit['type'] == 'globals':
                    if '<STRUCT>\n' not in response:
                        self.logger.log_failure("Response does not contain <STRUCT> tag. Trying again.")
                        continue
                    struct_trans = response.split('<STRUCT>\n')[1].split('</STRUCT>')[0]
                    # Remove any ```rust and ``` tags from struct_trans
                    struct_trans = struct_trans.replace('```rust', '').replace('```', '').strip()
                    return {
                        'struct': struct_trans,
                        'imports': imports,
                    }
                break
            except ModelException as e:
                self.logger.log_status("Model exception")
                self.logger.log_status(e)
                self.logger.log_status("Trying again")
                continue

        return None
    