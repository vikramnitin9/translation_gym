import argparse
import glob
import shutil
import traceback
import logging
import json
import sys
import re
import os
import stat
import subprocess

from typing import List, Dict, Sequence
from dataclasses import dataclass


@dataclass(frozen=True)
class Dependencies:
    """Dependencies for an object file."""
    source_file: str
    header_files: Sequence[str]


logging.basicConfig(
    stream=sys.stderr,  # Log messages to stderr
    level=logging.DEBUG,  # Set the logging level
    format="%(asctime)s - %(levelname)s - %(message)s"  # Format
)

_LOGGER = logging.getLogger("logger")

_INCLUDE_DIR = "include"

_MAKEFILE = f"""\
CFLAGS=-I./{_INCLUDE_DIR} -g
SRCS := $(wildcard *.c)
OBJS := $(SRCS:.c=.o)
LDFLAGS= -lcrypto -lssl -lgmp

.PHONY: all
all: clean {{program_name}}

{{program_name}}: $(OBJS)
	gcc $^ $(LDFLAGS) -o $@

%.o: %.c
	gcc $(CFLAGS) -c $< -o $@

.PHONY: clean
clean:
	rm -rf $(OBJS) {{program_name}}
"""


def _parse_arguments() -> Dict[str, str]:
    parser = argparse.ArgumentParser(description="Create C and Rust versions of"
                                     " a Coreutils program")

    parser.add_argument("--program_name",
                        type=str,
                        help="Name of the executable (i.e. echo, cat, etc.)",
          required=True)
    parser.add_argument("--corpus_dir",
                        type=str,
                        help="The parent directory that will contain the C and Rust versions",
                        default=os.getcwd())
    parser.add_argument("--coreutils_dir",
                        type=str,
                        help="The root directory for coreutils.",
                        required=True)

    args = parser.parse_args()

    return vars(args)


def _clean_deps(deps: List[str]) -> List[str]:
    return [dep.lstrip().rstrip() for dep in deps
            if len(dep)
            and not os.path.isabs(dep.lstrip().rstrip())
            and '\\' not in dep]


def _get_deps(filepath: str) -> Dependencies:
    directory = os.path.dirname(filepath)
    filename, _ = os.path.splitext(os.path.basename(filepath))
    deps_filepath = os.path.join(directory, ".deps", f"{filename}.Po")
    if (not os.path.exists(deps_filepath)
            or not os.path.isfile(deps_filepath)):
        _LOGGER.error(f"Could not find the dependency file: '{deps_filepath}'")
        sys.exit(1)
    with open(deps_filepath, 'r') as deps_file:
        lines = deps_file.readlines()
    if not len(lines):
        _LOGGER.error(f"The dependency file '{deps_filepath}' is empty.")
        sys.exit(1)
    # Find the line with the target
    found_target = False
    target = f"{filename}.o:"
    for i, line in enumerate(lines):
        if target in line:
            found_target = True
            break
    if not found_target:
        _LOGGER.error(
            f"Did not find the target '{target}' in the dependency file"
            " '{deps_filepath}' for '{filepath}'")
        sys.exit(1)
    deps_list = line.split(':')[1].split(' ')
    deps_list = _clean_deps(deps_list)
    for line in lines[i + 1:]:
        if ":" in line:
            # Found another target
            break
        deps_list += _clean_deps(line.split(' '))
    return Dependencies(
        source_file=deps_list[0], header_files=deps_list[1:])

def _copy(src: str, dest: str) -> None:
    new_path = shutil.copy(src, dest)
    mode = (
        stat.S_IRUSR
        | stat.S_IWUSR
        | stat.S_IRGRP
        | stat.S_IWGRP
        | stat.S_IROTH
        | stat.S_IWOTH
    )
    os.chmod(new_path, mode)


def _copy_header_file(header_filepath: str, dest_path: str) -> None:
    filepath_parts = header_filepath.split(os.sep)
    if "lib" in filepath_parts:
        i = filepath_parts.index("lib")
        dest_path = os.path.join(dest_path, *filepath_parts[i + 1:-1])
    elif "src" in filepath_parts:
        i = filepath_parts.index("src")
        dest_path = os.path.join(dest_path, *filepath_parts[i + 1:-1])
    if not os.path.exists(dest_path):
        try:
            os.makedirs(dest_path)
        except Exception as e:
            _LOGGER.error("Could not create the directory {dest_path}")
    _copy(header_filepath, dest_path)


def _copy_deps_to_dir(
    deps: Dependencies, out_c_dir: str, coreutils_dir: str) -> None:
    src_filepath = os.path.join(coreutils_dir, deps.source_file)
    if not os.path.isfile(src_filepath):
        _LOGGER.error(
            f"Could not find the file '{src_filepath}' to copy to '{out_c_dir}'")
        sys.exit(1)
    _copy(src_filepath, out_c_dir)
    include_path = os.path.join(out_c_dir, _INCLUDE_DIR)
    for header_file in deps.header_files:
        header_filepath = os.path.join(coreutils_dir, header_file)
        if not os.path.isfile(header_filepath):
            _LOGGER.warning(
                f"Could not find the file '{header_filepath}' to copy to"
                f" '{include_path}'")
            continue
        _copy_header_file(header_filepath, include_path)


def _run_command(command):
    try:
        output = subprocess.run(command,
                                shell=True, capture_output=True, text=True)
        return output.stdout.lstrip().rstrip(), output.returncode
    except subprocess.CalledProcessError as e:
        _LOGGER.error(f"Could not run the command '{command}':\n{str(e)}")


def _map_symbols_to_dependencies(dir_to_search: str) -> Dict[str, Dependencies]:
    syms_to_deps = dict()
    for object_filepath in glob.glob(
            os.path.join(dir_to_search, "**", "*.o"), recursive=True):
        symbols, _ = _run_command(
            "nm --defined-only --extern-only --format=\"just-symbols\""
            f" \"{object_filepath}\"")
        deps = _get_deps(object_filepath)
        for symbol in symbols.splitlines():
            if not len(symbol):
                continue
            syms_to_deps[symbol] = deps
    return syms_to_deps


def main():
    args = _parse_arguments()
    _LOGGER.info(
        f"Received the following arguments:\n{json.dumps(args, indent=4)}")

    if not os.path.exists(args["coreutils_dir"]):
        _LOGGER.error(
            f"The coreutils directory does not exist: {args['coreutils_dir']}")
        sys.exit(1)

    out_prog_dir = os.path.join(args["corpus_dir"], args["program_name"])
    out_c_dir = os.path.join(out_prog_dir, "c")
    out_rust_dir = os.path.join(out_prog_dir, "rust")

    if os.path.exists(out_prog_dir):
        try:
            shutil.rmtree(out_prog_dir)
        except Exception as e:
            _LOGGER.error(
                f"Could not remove the directory:\n{traceback.format_exc()}")
            sys.exit(1)

    try:
        os.makedirs(out_c_dir)
        os.makedirs(os.path.join(out_c_dir, _INCLUDE_DIR))
        os.makedirs(out_rust_dir)
    except Exception as e:
        _LOGGER.error(
            f"Could not create the output directories:\n"
            "{traceback.format_exc()}")
        sys.exit(1)

    _LOGGER.info(f"Created the '{out_c_dir}' and '{out_rust_dir}' directories")

    _run_command(f"make --directory={args['coreutils_dir']}")

    _LOGGER.info(f"Compiled the source files in {args['coreutils_dir']}")

    syms_to_deps = _map_symbols_to_dependencies(
        os.path.join(args["coreutils_dir"], "src"))
    syms_to_deps.update(
        _map_symbols_to_dependencies(
            os.path.join(args["coreutils_dir"], "lib")))

    _LOGGER.info("Mapped the symbols defined in coreutils to the dependencies."
                 f" Total number of symbols mapped: {len(syms_to_deps)}")

    c_makefile_path = os.path.join(out_c_dir, "Makefile")
    try:
        with open(c_makefile_path, 'w') as makefile:
            makefile.write(_MAKEFILE.format(program_name=args["program_name"]))
    except Exception as e:
        _LOGGER.error(
            f"Could not create the Makefile '{c_makefile_path}':\n"
            f"{traceback.format_exc()}")
        sys.exit(1)

    _LOGGER.info(f"Created the Makefile: '{c_makefile_path}'")

    main_source_filepath = os.path.join(
        args["coreutils_dir"], "src", f"{args['program_name']}.c")
    deps = _get_deps(main_source_filepath)
    _copy_deps_to_dir(deps, out_c_dir, args["coreutils_dir"])

    _LOGGER.info(f"Copied the initial dependencies to '{out_c_dir}'")

    _LOGGER.info("Copying the remaining .c and .h files...")

    undefined_symbol_pattern = r".*undefined reference to `(.*?)'"
    missing_file_pattern = r".* (.*): No such file or directory"
    i = 1
    while True:
        make_output, _ = _run_command(
            f"make -i --directory=\"{out_c_dir}\" {args['program_name']} 2>&1")
        missing_symbols = re.findall(undefined_symbol_pattern, make_output)
        missing_files = re.findall(missing_file_pattern, make_output)
        num_missing_symbols = len(missing_symbols)
        num_missing_files = len(missing_files)
        _LOGGER.info(f"Iteration {i}:"
                     f" Number of missing symbols: {num_missing_symbols};"
                     f" Number of missing files: {num_missing_files}")
        i += 1
        if not num_missing_symbols and not num_missing_files:
            break
        if num_missing_files:
            for file in missing_files:
                filepath, _ = _run_command(
                    f"find {args['coreutils_dir']} -type f -name \"{file}\""
                    " | head -n 1")
                if not os.path.isfile(filepath):
                    continue
                _copy_header_file(
                    filepath, os.path.join(out_c_dir, _INCLUDE_DIR))
        for missing_symbol in missing_symbols:
            if missing_symbol not in syms_to_deps:
                _LOGGER.error(
                    "Could not find the dependencies for the symbol"
                    f" '{missing_symbol}'")
                sys.exit(1)
            deps = syms_to_deps[missing_symbol]
            if not deps.source_file and not len(deps.header_files):
                _LOGGER.warning(
                    "Did not find dependencies for the missing symbol"
                    f" '{missing_symbol}'")
                continue
            _copy_deps_to_dir(deps, out_c_dir, args["coreutils_dir"])

    _LOGGER.info("Collected all the dependencies for the C version")

    mk_output, mk_retcode = _run_command(
        f"make --directory={out_c_dir} {args['program_name']} 2>&1")
    if mk_retcode:
        _LOGGER.error(
            f"Could not build the C version of {args['program_name']}:\n"
            f"{mk_output}"
        )
        sys.exit(1)

    _LOGGER.info(f"Successfully built the C version of {args['program_name']}")

    if shutil.which("intercept-build") is None:
        sb_install_cmd = "pip install scan-build"
        _LOGGER.warning(
            "The intercept-build command, needed to create the Rust version,"
            " was not found. Attempting to install with"
            f" `{sb_install_cmd}`"
        )
        pip_out, pip_retcode = _run_command(f"{sb_install_cmd} 2>&1")
        if pip_retcode:
            _LOGGER.error(f"Failed to run `{sb_install_cmd}`:\n{pip_out}")
            sys.exit(1)
        else:
            _LOGGER.info("Successfully installed intercept-build")

    _run_command(f"make -s --directory={out_c_dir} clean")
    compilation_db_filepath = os.path.join(out_c_dir, "compile_commands.json")
    ib_out, ib_retcode = _run_command(
        f"intercept-build --cdb {compilation_db_filepath}"
        f" make -s --directory={out_c_dir} {args['program_name']} 2>&1"
    )
    if ib_retcode:
        _LOGGER.error(f"Could not build the compilation database:\n{ib_out}")
        sys.exit(1)

    if shutil.which("c2rust") is None:
        _LOGGER.warning(
            "Did not find the c2rust command, attempting to install it")
        c2r_install_out, c2r_install_retcode = _run_command(
            "cargo install c2rust 2>&1")
        if c2r_install_retcode:
            _LOGGER.error(f"Could not install c2rust:\n{c2r_install_out}")
            sys.exit(1)
        _LOGGER.info("Successfully installed c2rust")

    c2r_out, c2r_retcode = _run_command(
        f"c2rust transpile --binary {args['program_name']} --overwrite-existing"
        f" --output-dir={out_rust_dir}"
        f" {compilation_db_filepath} 2>&1"
    )
    if c2r_retcode:
        _LOGGER.error(
            f"Could not transpile the program with c2rust:\n{c2r_out}")
        sys.exit(1)
    _LOGGER.info("Successfully transpiled the program with c2rust")

    # Allowing unused imports because the Rust auto fixer removes the
    # dependency on the library created to contain the helper functions
    main_rust_src_filepath = os.path.join(
        out_rust_dir, "src", f"{args['program_name']}.rs")
    unused_import_out, unused_import_retcode = _run_command(
        "sed -i 's/^\(#!\[allow([^\)]*\))/\\1, unused_imports)/'"
        f" {main_rust_src_filepath} 2>&1"
    )
    if unused_import_retcode:
        _LOGGER.error(
            "Could not add `unused_imports` to the #![allow(...)]` statement"
            f" in {main_rust_src_filepath}")
        sys.exit(1)

    # Removing `compile_error!()` calls
    rm_comp_err_out, rm_comp_err_retcode = _run_command(
        f"find {os.path.join(out_rust_dir, 'src')} -type f"
        f" -exec perl -0777 -pi -e 's/compile_error!\((.|\\n)*?\)//g' {{}} +"
    )
    if rm_comp_err_retcode:
        _LOGGER.error(
            "Could not remove the `compile_error!()` statements from the Rust"
            " source files")
        sys.exit(1)

    # Using cargo to automatically fix compile errors
    cg_fix_out, cg_fix_retcode = _run_command(
        f"cd {out_rust_dir}"
        " && cargo fix -Z unstable-options --keep-going --no-default-features"
        " --broken-code --allow-no-vcs --allow-dirty --allow-staged 2>&1 "
    )
    if cg_fix_retcode:
        _LOGGER.error(f"Running `cargo fix` failed:\n{cg_fix_out}")
        sys.exit(1)

    _LOGGER.info("Successfully cleaned the Rust version")

    cb_out, cb_retcode = _run_command(f"cd {out_rust_dir} && cargo build 2>&1")
    if cb_retcode:
        _LOGGER.error(f"Could not build the Rust version:\n{cb_out}")
        sys.exit(1)
    _LOGGER.info(
        f"Successfully built the Rust version of {args['program_name']}")

    _run_command(f"make --directory={out_c_dir} clean")
    _run_command(f"cd {out_rust_dir} && cargo clean")


if __name__ == "__main__":
    main()
