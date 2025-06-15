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
     'calledFunctions': [{'name': 'called function name',
                          'binding': 'Signature of Rust FFI binding',
                          'translated': 'Signature of translated version of function'}],
     'imports': ['import1', 'import2']}
    :return: The prompt string
    """

    if len(func['calledFunctions']) == 0:
        calledFunctionDesc = ""
    else:
        calledFunctionDesc = "This function calls the following functions:\n"
    for i, called_func in enumerate(func['calledFunctions']):
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

    globals_list = func.get("globals", [])
    if globals_list:
        items = []
        for g in globals_list:
            name = g["name"]
            ctype = g.get("type", "int")
            rust_ty = "libc::c_int" if ctype == "int" else ctype.replace("struct ", "")
            items.append(f"`{name}` (`{ctype}`) ⇒ `&mut {rust_ty}`")
        globalDesc = (
            "This function references the following C globals and **must** be translated into\n"
            "a Rust function taking a mutable borrow of each (never call the C FFI version):\n\n"
            " - " + "\n - ".join(items) + "\n\n"
            "In your function signature, name each parameter `*_ref` (e.g. `global_counter_ref`) to avoid\n"
            "shadowing the `static mut global_counter`, and always pass that same reference into any\n"
            "`*_rust` callee.\n\n"
        )
    else:
        globalDesc = ""

    # if the C function has no arguments but uses globals
    if func.get("num_args", 0) == 0 and globals_list:
        wrapperDesc = (
            "Since this C function has no parameters, your wrapper must also take no parameters.  "
            "Inside that zero-arg wrapper, `unsafe`-borrow the `static mut` globals and pass them to the `<FUNC>` "
            "implementation.  For example:\n\n"
            "```rust\n"
            "#[no_mangle]\n"
            "pub extern \"C\" fn {name}() -> libc::c_int {{\n"
            "    unsafe {{ {name}_rust(&mut global_counter) }}\n"
            "}}\n"
            "```\n\n"
        ).format(name=func["name"])
    else:
        wrapperDesc = ""

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
As far as possible, avoid raw pointers and unsafe function calls, and use only safe Rust. Also avoid libc types and use Rust native types instead.
You can assume that all the structures and global variables already have definitions in Rust, and you do not need to redefine them.
Do not use any dummy code like "// Full implementation goes here", etc. All the code you write will be substituted directly into the codebase without a human reviewing it. So it should be functional and complete.
Feel free to change the function signature and modify the function body as needed.
If you need imports, you can add them in the <IMPORTS>...</IMPORTS> section. Do not provide them along with the function body.
{importDesc}
{globalDesc}

Also provide a wrapper function that calls this function.
The wrapper function should have the *same* arguments and return type as the C function, except with C types replaced with their corresponding libc crate types.
For example, replace `int` with `libc::c_int`, `char*` with `*mut libc::c_char`, etc.
Also remember to use `#[no_mangle]` and `pub extern "C" fn ...` for the wrapper function.
{wrapperDesc} 

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
pub extern "C" fn {func['name']} ...
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

    prompt = f'''Translate the following C struct to idiomatic Rust:
```c
{struct['source']}
```
The struct should be a `struct` in Rust, and the fields should be `pub` so that they can be accessed from outside the module.
The name of the Rust struct should be `{struct['name']}_rust`.
Make sure to use idiomatic Rust types instead of C types. Avoid libc types and use Rust native types instead.
For example, replace `int` with `i32`, `char*` with `String`, etc.
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