from translation_gym.models import get_model_from_name, ModelException
from translation_gym.helpers import *

class Translator:

    def __init__(self, model):
        self.model = get_model_from_name(model)
        self.conversation = []

    def construct_prompt_for_func(self, func):

        prompt = f'''Translate the following C function to idiomatic Rust:
```c
{func['body']}
```
As far as possible, use only safe Rust. Avoid raw pointers and unsafe function calls.
You can assume that all the structures and global variables already have definitions in Rust, and you do not need to redefine them.
Do not use any dummy code like "// Full implementation goes here", etc. All the code you write will be substituted directly into the codebase without a human reviewing it. So it should be functional and complete.
Feel free to change the function signature and modify the function body as needed.
If you need imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the function body.

Also provide a wrapper function that calls this function.
The wrapper function should have the *same* arguments and return type as the C function, except with C types replaced with their corresponding libc crate types.
For example, replace `int` with `libc::c_int`, `char*` with `*mut libc::c_char`, etc.
Also remember to use `#[no_mangle]` and `pub extern "C" fn ...` for the wrapper function.

The name of the Rust function should be `{func['name']}_rust` and the wrapper function should be `{func['name']}`.

Follow this format:

<IMPORTS>
Any imports you need for {func['name']}_rust and {func['name']}
</IMPORTS>

<FUNC>
fn {func['name']}_rust ...
</FUNC>

<WRAPPER>
#[no_mangle]
pub extern "C" fn {func['name']} ...
</WRAPPER>
'''
        return prompt
    
    def translate(self, func, source_manager, verbose=False):

        body = source_manager.extract_body(func)
        func['body'] = body

        translation_prompt = self.construct_prompt_for_func(func)

        self.conversation = [{'role': 'system', 'content': 'You are an intelligent code assistant'},
                            {'role': 'user', 'content': translation_prompt.strip()}]

        while True:
            try:
                prCyan("Calling LLM for translation")
                response = self.model.gen(self.conversation, top_k=1, temperature=0)[0]
                self.conversation.append({'role': 'assistant', 'content': response})
                prGreen("LLM response received")
                if verbose:
                    prLightGray(response)
                # Parse the response and extract the text between either
                # <FUNC>...</FUNC>, <IMPORT>...</IMPORT> or <WRAPPER>...</WRAPPER> tags
                if '<IMPORTS>\n' in response:
                    imports = response.split('<IMPORTS>\n')[1].split('</IMPORTS>')[0]
                else:
                    imports = ''
                
                if '<FUNC>\n' not in response:
                    prRed("Response does not contain <FUNC> tag. Trying again.")
                    continue
                if '<WRAPPER>\n' not in response:
                    prRed("Response does not contain <WRAPPER> tag. Trying again.")
                    continue
                function_trans = response.split('<FUNC>\n')[1].split('</FUNC>')[0]
                wrapper = response.split('<WRAPPER>\n')[1].split('</WRAPPER>')[0]

                # Remove any ```rust and ``` tags from imports, function_trans and wrapper
                imports = imports.replace('```rust', '').replace('```', '').strip()
                function_trans = function_trans.replace('```rust', '').replace('```', '').strip()
                wrapper = wrapper.replace('```rust', '').replace('```', '').strip()
                break
            except ModelException as e:
                prCyan("Model exception")
                prCyan(e)
                prCyan("Trying again")
                continue

        return {
            'func': function_trans,
            'wrapper': wrapper,
            'imports': imports,
        }
    
    def repair(self, result, source_manager, verbose=False):

        assert len(self.conversation) > 0, "Repair called before translation"

        if result['category'] == "Compile Error":
            prompt = ("The translation generated the following compile error:\n"
                    f"{result['message']}\n"
                    f"Please re-generate the translation of the function, wrapper function, and imports. "
                    f"Remember to follow the same format with <IMPORTS></IMPORTS>, <FUNC></FUNC>, and <WRAPPER></WRAPPER> tags.")
        elif result['category'] == "Test Failure":
            prompt = ("The translation failed tests. This was the command output:\n"
                    f"{result['message']}\n"
                    f"Please re-generate the translation of the function, wrapper function, and imports. "
                    f"Remember to follow the same format with <IMPORTS></IMPORTS>, <FUNC></FUNC>, and <WRAPPER></WRAPPER> tags.")
        else:
            raise NotImplementedError("Repair not implemented for this error type")
        
        self.conversation += [{'role': 'user', 'content': prompt.strip()}]

        while True:
            try:
                prCyan("Calling LLM for repair")
                response = self.model.gen(self.conversation, top_k=1, temperature=0)[0]
                self.conversation.append({'role': 'assistant', 'content': response})
                prGreen("LLM response received")
                if verbose:
                    prLightGray(response)
                # Parse the response and extract the text between either
                # <FUNC>...</FUNC>, <IMPORT>...</IMPORT> or <WRAPPER>...</WRAPPER> tags
                if '<IMPORTS>\n' in response:
                    imports = response.split('<IMPORTS>\n')[1].split('</IMPORTS>')[0]
                else:
                    imports = ''
                
                if '<FUNC>\n' not in response:
                    prRed("Response does not contain <FUNC> tag. Trying again.")
                    continue
                if '<WRAPPER>\n' not in response:
                    prRed("Response does not contain <WRAPPER> tag. Trying again.")
                    continue
                function_trans = response.split('<FUNC>\n')[1].split('</FUNC>')[0]
                wrapper = response.split('<WRAPPER>\n')[1].split('</WRAPPER>')[0]

                # Remove any ```rust and ``` tags from imports, function_trans and wrapper
                imports = imports.replace('```rust', '').replace('```', '').strip()
                function_trans = function_trans.replace('```rust', '').replace('```', '').strip()
                wrapper = wrapper.replace('```rust', '').replace('```', '').strip()
                break
            except ModelException as e:
                prCyan("Model exception")
                prCyan(e)
                prCyan("Trying again")
                continue

        return {
            'func': function_trans,
            'wrapper': wrapper,
            'imports': imports,
        }



