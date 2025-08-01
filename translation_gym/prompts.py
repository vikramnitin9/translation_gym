"""
This file constructs the prompts used by `DefaultTranslator`
for translating C to Rust.
"""

def construct_prompt_for_func(func):

    """
    Constructs the prompt for translating a C function to Rust.
    :param func: The function to be translated
    {'source': 'C function body',
     'name': 'function name',
     'functions': [{'name': 'called function name',
                    'binding': 'Signature of Rust FFI binding',
                    'translated': 'Signature of translated version of function'}],
     'structs': [{'name': 'struct name',
                  'binding': 'Rust FFI binding',
                  'translated': 'Translated version of struct'}],
     'enums':   [{'name': 'enum name',
                  'binding': 'Rust FFI binding',
                  'translated': 'Translated version of enum'}],
     'globals': [{'name': 'global variable name',
                  'binding': 'Rust FFI binding',
                  'translated': 'Translated version of global variable'}],
     'imports': ['import1', 'import2']}
    :return: The prompt string
    """

    if len(func['functions']) == 0:
        calledFunctionDesc = ""
    else:
        calledFunctionDesc = "This function calls the following functions:\n"
    for i, called_func in enumerate(func['functions']):
        if 'translated' in called_func:
            calledFunctionDesc += f"{i+1}. {called_func['name']}. This has a Rust reimplementation, with this signature:\n"
            calledFunctionDesc += f"```rust\n{called_func['translated']}\n```\n"
        elif 'binding' in called_func:
            calledFunctionDesc += f"{i+1}. {called_func['name']}. This has a Rust FFI binding to a C implementation, with this signature:\n"
            calledFunctionDesc += f"```rust\n{called_func['binding']}\n```\n"
            calledFunctionDesc += "Note that you will need to use the `unsafe` keyword to call this function. If you can come up with a native safe Rust alternative for this function, you should use that instead.\n"
        else:
            calledFunctionDesc += f"{i+1}. {called_func['name']}. This function is not accessible to you, so you need to use a substitute.\n"
    
    if len(func['structs']) == 0:
        structDesc = ""
    else:
        structDesc = "This function uses the following structs:\n"
    for i, struct in enumerate(func['structs']):
        if 'translated' in struct:
            structDesc += f"{i+1}. {struct['name']}. This has a Rust reimplementation, with this definition:\n"
            structDesc += f"```rust\n{struct['translated']}\n```\n"
        elif 'binding' in struct:
            structDesc += f"{i+1}. {struct['name']}. This has a Rust FFI binding to a C implementation, with this definition:\n"
            structDesc += f"```rust\n{struct['binding']}\n```\n"
        else:
            structDesc += f"{i+1}. {struct['name']}. This struct is not accessible to you, so you need to use a substitute.\n"

    if len(func['enums']) == 0:
        enumDesc = ""
    else:
        enumDesc = "This function uses the following enums:\n"
    for i, enum in enumerate(func['enums']):
        if 'translated' in enum:
            enumDesc += f"{i+1}. {enum['name']}. This has a Rust reimplementation, with this definition:\n"
            enumDesc += f"```rust\n{enum['translated']}\n```\n"
        elif 'binding' in enum:
            enumDesc += f"{i+1}. {enum['name']}. This has a Rust FFI binding to a C implementation, with this definition:\n"
            enumDesc += f"```rust\n{enum['binding']}\n```\n"
        else:
            enumDesc += f"{i+1}. {enum['name']}. This enum is not accessible to you, so you need to use a substitute.\n"

    if len(func['globals']) == 0:
        globalDesc = ""
        globalWrapperDesc = ""
    else:
        globalDesc = "This function (or one of its callees) uses the following global variables:\n"
    for i, glob in enumerate(func['globals']):
        if 'translated' in glob:
            globalDesc += f"{i+1}. {glob['name']}. This can be replaced by an object of this struct:\n"
            globalDesc += f"```rust\n{glob['translated']}\n```\n"
            globalDesc += "This struct has `get` and `set` methods to interact with the field.\n"
            globalDesc += "The struct also has a `new` method that creates a new instance of the struct, "
            globalDesc += "and initializes it with the current value of the global variable in C. So you do not need to provide it with an initial value.\n"
            globalDesc += "Likewise, the `get` and `set` methods also automatically synchronize with the global variable in C, so you do not need to worry about that.\n"
            globalDesc += "The translated function should take an object of this struct as one of its arguments.\n"
        elif 'binding' in glob:
            globalDesc += f"{i+1}. {glob['name']}. This is accessible through its Rust FFI binding to C, with this declaration:\n"
            globalDesc += f"```rust\n{glob['binding']}\n```\n"
            globalDesc += "Note that you will need to use the `unsafe` keyword to access this binding.\n"
        
        globalWrapperDesc = "The wrapper function should also take care of passing the structures corresponding to global variables, as arguments to the translated function.\n"
        globalWrapperDesc += "It should call the `new` method of the global variable's struct with no arguments, and pass the resulting object to the translated function.\n"

    if len(func['imports']) == 0:
        importDesc = ""
    else:
        importDesc = f"The Rust file where this function will be inserted already has the following imports:\n```rust\n{'\n'.join(func['imports'])}\n```\n"
        importDesc += "Do not repeat them in the <IMPORTS>...</IMPORTS> section, otherwise this will lead to duplicate imports.\n"

    prompt = f'''Translate the following C function to idiomatic Rust:
```c
{func['source']}
```
{calledFunctionDesc}
{structDesc}
{enumDesc}
{globalDesc}
As far as possible, avoid raw pointers and unsafe function calls, and use only safe Rust. Also avoid libc types and use Rust native types instead.
Avoid using bindings to C functions and structs as far as possible, and use the Rust reimplementations if available.
Do not use any dummy code like "// Full implementation goes here", etc. All the code you write will be substituted directly into the codebase without a human reviewing it. So it should be functional and complete.
Feel free to change the function signature and modify the function body as needed.
If you need imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the function body.
{importDesc}

Also provide a wrapper function that calls this function.
The wrapper function should have the *same* arguments and return type as the C function, except with C types replaced with their corresponding libc crate types or FFI bindings.
For example, replace `int` with `libc::c_int`, `char*` with `*mut libc::c_char`, etc.
{globalWrapperDesc}
Make sure to use `#[no_mangle]` and `pub unsafe extern "C" fn ...` for the wrapper function.
Note that the wrapper function can use unsafe code to access FFI bindings to globals, structs, etc. Make sure that the wrapper function actually calls the translated function!

The name of the Rust function should be `{func['name']}_rust` and the wrapper function should be `{func['name']}`.

Follow this format:

<IMPORTS>
Any imports you need for {func['name']}_rust and {func['name']}. Can be empty.
</IMPORTS>

<FUNC>
fn {func['name']}_rust ...
</FUNC>

<WRAPPER>
#[no_mangle]
pub unsafe extern "C" fn {func['name']} ...
</WRAPPER>
'''
    return prompt


def construct_prompt_for_struct(struct):
    """
    Constructs the prompt for translating a C struct to Rust.
    :param struct: The struct to be translated
    {'source': 'C struct body',
     'name': 'struct name'}
    :return: The prompt string
    """

    if len(struct['imports']) == 0:
        importDesc = ""
    else:
        importDesc = f"The Rust file where this struct will be inserted already has the following imports:\n```rust\n{'\n'.join(struct['imports'])}\n```\n"
        importDesc += "Do not repeat them in the <IMPORTS>...</IMPORTS> section, otherwise this will lead to duplicate imports.\n"

    if len(struct['structs']) == 0 and len(struct['enums']) == 0:
        typeDesc = ""
    else:
        typeDesc = "This structure uses the following subtypes:\n"
    for i, subtype in enumerate(struct.get('structs', []) + struct.get('enums', [])):
        if 'translated' in subtype:
            typeDesc += f"{i+1}. {subtype['name']}. This has a Rust reimplementation, with this definition:\n"
            typeDesc += f"```rust\n{subtype['translated']}\n```\n"
        elif 'binding' in subtype:
            typeDesc += f"{i+1}. {subtype['name']}. This has a Rust FFI binding to a C implementation, with this definition:\n"
            typeDesc += f"```rust\n{subtype['binding']}\n```\n"
        else:
            typeDesc += f"{i+1}. {subtype['name']}. This subtype is not accessible to you, so you need to use a substitute.\n"

    prompt = f'''Translate the following C struct to idiomatic Rust:
```c
{struct['source']}
```
The struct should be a `struct` in Rust, and the fields should be `pub` so that they can be accessed from outside the module.
The name of the Rust struct should be `{struct['name']}_rust`.
Make sure to use idiomatic Rust types instead of C types. Avoid libc types and use Rust native types instead.
For example, replace `int` with `i32`, `char*` with `String`, etc.
{typeDesc}
If you need imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the struct source.
{importDesc}

Follow this format:
<IMPORTS>
Any imports you need for {struct['name']}_rust. Can be empty.
</IMPORTS>
<STRUCT>
struct {struct['name']}_rust ...
</STRUCT>
'''
    return prompt


def construct_prompt_for_global(glob):
    """
    Constructs the prompt for translating a C global variable to Rust.
    :param glob: The global variable to be translated
    {'source': 'C global variable definition',
     'name': 'global variable name'}
    :return: The prompt string
    """

    if len(glob['imports']) == 0:
        importDesc = ""
    else:
        importDesc = f"The Rust file where this global will be inserted already has the following imports:\n```rust\n{'\n'.join(glob['imports'])}\n```\n"
        importDesc += "Do not repeat them in the <IMPORTS>...</IMPORTS> section, otherwise this will lead to duplicate imports.\n"

    prompt = f'''Here is a declaration of a global variable in Rust:
```rust
{glob['binding']}
```
However, this uses non-idiomatic Rust types. We want an idiomatic Rust equivalent, that does not use libc types, raw pointers, or `static mut`.
Your task is to create a struct that provides an idiomatic type interface for interacting with the global variable.
The struct should be named `{''.join([s.capitalize() for s in glob['name'].split('_')])}Wrapper`. It has one field, `val: T`.
It should implement 3 methods:
    1. `get(&self) -> T`: A method that reads the value of the global variable (NOT `val`!), using unsafe code if necessary, converts it to an idiomatic type `T`, and returns it.
    2. `set(&mut self, val: T)`: A method that takes a value of the idiomatic type T as an argument, and assigns it to the `val` field.
        It also converts `val` to the type of the global variable, and (if necessary) uses `unsafe` code internally to write it to the global variable.
        The global variable and the struct field should always be in sync.
    3. `new() -> Self`: Reads the current value of the global variable (using unsafe code if necessary), converts it to an idiomatic type `T` (very similar to `get`), and returns a new instance of this structure with `val` initialized to this value.
Through these two functions, the wrapper should ensure that the global variable and struct field are always in sync.
Note that `T` here is a placeholder for the actual idiomatic equivalent type. Avoid libc types and raw pointers for `T`. For example, replace `libc::c_int` with `i32`, `*mut libc::c_char` with `String`, etc.
If you need new imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the struct source.
{importDesc}
Follow this format:
<IMPORTS>
Any imports you need for `{''.join([s.capitalize() for s in glob['name'].split('_')])}Wrapper`. Can be empty.
</IMPORTS>
<STRUCT>
pub struct {''.join([s.capitalize() for s in glob['name'].split('_')])}Wrapper {{
    val: ...
}}
impl {''.join([s.capitalize() for s in glob['name'].split('_')])}Wrapper {{
...
}}
</STRUCT>
'''
    return prompt


def construct_prompt_for_enum(enum):
    """
    Constructs the prompt for translating a C enum to Rust.
    :param enum: The enum to be translated
    {'source': 'C enum body',
     'name': 'enum name'}
    :return: The prompt string
    """

    if len(enum['imports']) == 0:
        importDesc = ""
    else:
        importDesc = f"The Rust file where this enum will be inserted already has the following imports:\n```rust\n{'\n'.join(enum['imports'])}\n```\n"
        importDesc += "Do not repeat them in the <IMPORTS>...</IMPORTS> section, otherwise this will lead to duplicate imports.\n"

    prompt = f'''Translate the following C enum to idiomatic Rust:
```c
{enum['source']}
```
The name of the Rust enum should be `{enum['name'].capitalize()}_rust`. The names of the variants should be the same as the C enum variants, except with the first letter capitalized (if it is not already).
Make sure to use idiomatic Rust types instead of C types. Avoid libc types and use Rust native types instead.
For example, replace `int` with `i32`, `char*` with `String`, etc.
If you need imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the enum source.
{importDesc}

Follow this format:
<IMPORTS>
Any imports you need for {enum['name'].capitalize()}_rust. Can be empty.
</IMPORTS>
<ENUM>
#[derive(Clone, Copy, PartialEq, Eq)]
enum {enum['name'].capitalize()}_rust ...
</ENUM>
'''
    return prompt